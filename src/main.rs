mod data;
mod honeycomb;
mod semconv;

use std::{collections::HashMap, path};

use anyhow::Context;
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Path, State},
    http::header::HeaderMap,
    response::Response,
    routing::get,
    Router,
};
use chrono::Utc;
use clap::Parser;
use data::Node;
use futures::stream::{FuturesUnordered, StreamExt};
use honeycomb::HoneyComb;
use semconv::{Attribute, Examples, SemanticConventions};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    node: String,
}

#[derive(Template)]
#[template(path = "node.html")]
struct NodeTemplate {
    level: String,
    level_parts: Vec<String>,
    level_links: Vec<String>,
    nodes: Vec<Node<Attribute>>,
}

#[derive(Template)]
#[template(path = "usedby.html")]
struct UsedByTemplate {
    attribute: String,
    datasets: Vec<String>,
}

#[derive(Clone)]
struct AppState {
    db: Node<Attribute>,
    hc: Option<HoneyComb>,
}

#[derive(Parser, Debug)]
#[command(author, version)]
/// Honey Explore
///
/// Explore OpenTelemetry Semantic Convention compatible models in a web browser.
struct Args {
    /// Model paths
    ///
    /// Provide one or more paths to the root of semantic convention
    /// model directories. The path should be prefixed with a nickname
    /// followed by a double colon. For example:
    ///    otel::/Users/jerbly/Documents/code/public/semantic-conventions/model
    #[arg(short, long, required = true, num_args(1..))]
    model: Vec<String>,

    /// Address
    ///
    /// TCP Address to listen on.
    #[arg(short, long, default_value_t = String::from("127.0.0.1:3000"))]
    addr: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // load configuration
    dotenv::dotenv().ok();
    let args = Args::parse();
    let mut root_dirs = vec![];
    for path in args.model {
        if !path.contains("::") {
            anyhow::bail!("path must be prefixed with a nickname followed by a double colon");
        }
        let split = path.split("::").collect::<Vec<_>>();
        let nickname = split[0];
        let p = path::Path::new(&split[1]);
        if !p.is_dir() {
            anyhow::bail!("{} is not directory", path);
        }
        root_dirs.push((
            nickname.to_owned(),
            p.canonicalize()?
                .to_str()
                .context("invalid path")?
                .to_owned(),
        ));
    }

    let sc = SemanticConventions::new(&root_dirs)?;
    let mut root = Node::new("root".to_string(), None);
    let mut keys: Vec<_> = sc.attribute_map.keys().collect();
    keys.sort();

    // fetch all the honeycomb data and build a map of attribute name to datasets
    let hc = get_honeycomb().await?;
    if let Some(hc) = &hc {
        let attributes_used_by_datasets = get_attributes_used_by_datasets(hc, &sc).await?;
        for k in keys {
            let mut attribute = sc.attribute_map[k].clone();
            if let Some(datasets) = attributes_used_by_datasets.get(k) {
                let mut datasets = datasets.clone();
                datasets.sort();
                attribute.used_by = Some(datasets);
            }
            root.add_node(k, Some(attribute));
        }
    } else {
        for k in keys {
            root.add_node(k, Some(sc.attribute_map[k].clone()));
        }
    }
    let state = AppState { db: root, hc };

    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/node/:name", get(node_handler))
        .route("/usedby/:name", get(used_by_handler))
        .route("/hnyexists/:dataset/:column", get(honeycomb_exists_handler))
        .with_state(state);

    // run it
    let listener = tokio::net::TcpListener::bind(args.addr).await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_attributes_used_by_datasets(
    hc: &HoneyComb,
    sc: &SemanticConventions,
) -> anyhow::Result<HashMap<String, Vec<String>>> {
    let now = Utc::now();
    let mut datasets = hc
        .list_all_datasets()
        .await?
        .iter()
        .filter_map(|d| {
            if (now - d.last_written_at.unwrap_or(now)).num_days() < 60 {
                Some(d.slug.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    datasets.sort();

    let mut attributes_used_by_datasets: HashMap<String, Vec<String>> = HashMap::new();

    let mut tasks = FuturesUnordered::new();

    for dataset in datasets {
        let dataset_clone = dataset.clone();
        let hc_clone = hc.clone();
        tasks.push(async move {
            let columns = hc_clone.list_all_columns(&dataset_clone).await;
            match columns {
                Ok(columns) => (dataset_clone, columns),
                Err(e) => {
                    eprintln!(
                        "error fetching columns for dataset {}: {}",
                        dataset_clone, e
                    );
                    (dataset_clone, vec![])
                }
            }
        });
    }

    while let Some((dataset, columns)) = tasks.next().await {
        println!("mapping columns for dataset: {}", dataset);
        for column in columns {
            if sc.attribute_map.contains_key(&column.key_name) {
                let datasets = attributes_used_by_datasets
                    .entry(column.key_name.clone())
                    .or_insert(vec![]);
                datasets.push(dataset.clone());
            }
        }
    }

    // for dataset in datasets {
    //     println!("fetching columns for dataset: {}", dataset);
    //     let columns = hc.list_all_columns(&dataset).await?;
    //     for column in columns {
    //         if sc.attribute_map.contains_key(&column.key_name) {
    //             let datasets = attributes_used_by_datasets
    //                 .entry(column.key_name.clone())
    //                 .or_insert(vec![]);
    //             datasets.push(dataset.clone());
    //         }
    //     }
    // }

    Ok(attributes_used_by_datasets)
}

async fn get_honeycomb() -> anyhow::Result<Option<HoneyComb>> {
    let hc = match HoneyComb::new() {
        Ok(hc) => {
            let auth = hc.list_authorizations().await?;
            let required_access = ["columns", "createDatasets", "queries"];
            if auth.has_required_access(&required_access) {
                Some(hc)
            } else {
                eprintln!(
                    "continuing without honeycomb: missing required access {:?}:\n{}",
                    required_access, auth
                );
                None
            }
        }
        Err(e) => {
            eprintln!("continuing without honeycomb: {}", e);
            None
        }
    };
    Ok(hc)
}

async fn handler() -> impl IntoResponse {
    IndexTemplate {
        node: "root".to_owned(),
    }
}

async fn used_by_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut datasets = vec![];
    if let Some(node) = state.db.get_node(&name) {
        if let Some(attribute) = &node.value {
            if let Some(used_by) = &attribute.used_by {
                datasets.extend_from_slice(used_by);
            }
        }
    }
    UsedByTemplate {
        attribute: name,
        datasets,
    }
}

fn get_links(names: &Vec<String>) -> Vec<String> {
    // progressively join each name part to the previous
    let mut links = vec![];
    let mut prev = String::new();
    for name in names {
        if prev.is_empty() {
            prev = name.clone();
        } else {
            prev = format!("{}.{}", prev, name);
        }
        links.push(prev.clone());
    }
    links
}

async fn honeycomb_exists_handler(
    State(state): State<AppState>,
    Path((dataset, column)): Path<(String, String)>,
) -> Response {
    match &state.hc {
        None => "".into_response(),
        Some(hc) => {
            if let Ok(exists) = hc.get_exists_query_url(&dataset, &column).await {
                ([("HX-Redirect", exists)], "").into_response()
            } else {
                "".into_response()
            }
        }
    }
}

async fn node_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Response {
    // check headers to see if this is a full page request or an ajax request
    let hx_history_restore_request = headers
        .get("HX-History-Restore-Request")
        .and_then(|value| value.to_str().ok())
        .map(|s| s == "true")
        .unwrap_or(false);

    if hx_history_restore_request || !headers.contains_key("HX-Request") {
        // Handle the case where HX-History-Restore-Request is true
        return IndexTemplate { node: name }.into_response();
    }

    if name == "root" {
        return NodeTemplate {
            level: name.clone(),
            level_parts: vec![name.clone()],
            level_links: vec![name.clone()],
            nodes: state
                .db
                .children
                .values()
                .cloned()
                .collect::<Vec<Node<Attribute>>>(),
        }
        .into_response();
    }
    if name.starts_with("root.") {
        let name = name.trim_start_matches("root.");
        if let Some(node) = state.db.get_node(name) {
            let level_parts = name.split('.').map(|s| s.to_owned()).collect();
            let level_links = get_links(&level_parts);
            return NodeTemplate {
                level: name.to_owned(),
                level_parts,
                level_links,
                nodes: node
                    .children
                    .values()
                    .cloned()
                    .collect::<Vec<Node<Attribute>>>(),
            }
            .into_response();
        }
    }
    let level_parts = name.split('.').map(|s| s.to_owned()).collect();
    let level_links = get_links(&level_parts);
    if let Some(node) = state.db.get_node(&name) {
        NodeTemplate {
            level: name.clone(),
            level_parts,
            level_links,
            nodes: node
                .children
                .values()
                .cloned()
                .collect::<Vec<Node<Attribute>>>(),
        }
        .into_response()
    } else {
        NodeTemplate {
            level: name.clone(),
            level_parts,
            level_links,
            nodes: vec![],
        }
        .into_response()
    }
}

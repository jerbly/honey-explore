mod data;
mod semconv;

use std::{collections::BTreeMap, path, vec};

use anyhow::Context;
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Path, State},
    http::{
        header::{self, HeaderMap},
        StatusCode, Uri,
    },
    response::Response,
    routing::get,
    Router,
};
use clap::Parser;
use data::Node;
use honeycomb_client::honeycomb::HoneyComb;
use rust_embed::RustEmbed;
use semconv::{Attribute, Examples, PrimitiveType, SemanticConventions, Type::Simple};

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

#[derive(Template)]
#[template(path = "suffix_usedby.html")]
struct SuffixUsedByTemplate {
    attribute: String,
    suffix: String,
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

#[derive(RustEmbed)]
#[folder = "static/"]
struct Asset;
pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match Asset::get(path.as_str()) {
            Some(content) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            }
            None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        }
    }
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.starts_with("dist/") {
        path = path.replace("dist/", "");
    }

    StaticFile(path)
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
    // load semantic conventions
    let mut sc = SemanticConventions::new(&root_dirs)?;

    // build the tree
    let mut root = Node::new("root".to_string(), "".to_owned(), None);
    let hc = match honeycomb_client::get_honeycomb(&["columns", "createDatasets", "queries"]).await
    {
        Ok(hclient) => hclient,
        Err(e) => {
            eprintln!("Failed to get honeycomb client: {}", e);
            None
        }
    };

    // if we have a valid api-key with enough access permission then
    // fetch all the honeycomb data and augment the attributes
    if let Some(client) = &hc {
        add_hny_to_attributes(client, &mut sc).await?;
    }

    // add all the attributes to the tree
    let mut keys: Vec<_> = sc.attribute_map.keys().collect();
    keys.sort();
    for k in keys {
        root.add_node(k, Some(sc.attribute_map[k].clone()));
    }
    // print the tree
    root.print(0, false);

    let state = AppState { db: root, hc };

    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/tree/:name", get(tree_handler))
        .route("/node/:name", get(node_handler))
        .route("/usedby/:name", get(used_by_handler))
        .route("/suffix_usedby/:name/:suffix", get(suffix_used_by_handler))
        .route(
            "/hnyexists/:dataset/:column/:suffix",
            get(honeycomb_exists_handler),
        )
        .route("/dist/*file", get(static_handler))
        .with_state(state);

    // run it
    let listener = tokio::net::TcpListener::bind(args.addr).await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn add_hny_to_attributes(hc: &HoneyComb, sc: &mut SemanticConventions) -> anyhow::Result<()> {
    let dataset_slugs = hc.get_dataset_slugs(30, None).await?;
    eprint!("Reading {} datasets ", dataset_slugs.len());
    hc.process_datasets_columns(30, &dataset_slugs, |dataset, columns| {
        eprint!(".");
        for column in columns {
            if let Some(attribute) = sc.attribute_map.get_mut(&column.key_name) {
                match attribute.used_by {
                    Some(ref mut used_by) => used_by.push(dataset.clone()),
                    None => attribute.used_by = Some(vec![dataset.clone()]),
                }
            } else {
                // Handle template types:
                // Extract the suffix from the end and see if the prefix is a known attribute
                if let Some((prefix, suffix)) = column.key_name.rsplit_once('.') {
                    if let Some(attribute) = sc.attribute_map.get_mut(prefix) {
                        if attribute.is_template_type() {
                            let suffixes = match attribute.template_suffixes {
                                Some(ref mut suffixes) => suffixes,
                                None => {
                                    attribute.template_suffixes = Some(BTreeMap::new());
                                    attribute.template_suffixes.as_mut().unwrap()
                                }
                            };
                            suffixes
                                .entry(suffix.to_owned())
                                .and_modify(|datasets| datasets.push(dataset.clone()))
                                .or_insert(vec![dataset.clone()]);
                        }
                    }
                }
            }
        }
    })
    .await?;
    eprintln!();
    Ok(())
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

async fn suffix_used_by_handler(
    State(state): State<AppState>,
    Path((name, suffix)): Path<(String, String)>,
) -> impl IntoResponse {
    let mut datasets = vec![];
    if let Some(node) = state.db.get_node(&name) {
        if let Some(attribute) = &node.value {
            if let Some(suffixes) = &attribute.template_suffixes {
                if let Some(used_by) = suffixes.get(&suffix) {
                    datasets.extend_from_slice(used_by);
                }
            }
        }
    }
    SuffixUsedByTemplate {
        attribute: name,
        suffix,
        datasets,
    }
}

async fn honeycomb_exists_handler(
    State(state): State<AppState>,
    Path((dataset, column, suffix)): Path<(String, String, String)>,
) -> Response {
    if let Some(hc) = &state.hc {
        if let Some(node) = state.db.get_node(&column) {
            if let Some(value) = node.value.as_ref() {
                if let Some(column_type) = &value.r#type {
                    match column_type {
                        Simple(PrimitiveType::Int) | Simple(PrimitiveType::Double) => {
                            if let Ok(avg) = hc.get_avg_query_url(&dataset, &column).await {
                                return ([("HX-Redirect", avg)], "").into_response();
                            }
                        }
                        Simple(PrimitiveType::TemplateOfInt)
                        | Simple(PrimitiveType::TemplateOfDouble) => {
                            let column_with_suffix = format!("{}.{}", column, suffix);
                            if let Ok(avg) =
                                hc.get_avg_query_url(&dataset, &column_with_suffix).await
                            {
                                return ([("HX-Redirect", avg)], "").into_response();
                            }
                        }
                        Simple(PrimitiveType::TemplateOfString)
                        | Simple(PrimitiveType::TemplateOfBoolean)
                        | Simple(PrimitiveType::TemplateOfArrayOfString)
                        | Simple(PrimitiveType::TemplateOfArrayOfInt)
                        | Simple(PrimitiveType::TemplateOfArrayOfDouble)
                        | Simple(PrimitiveType::TemplateOfArrayOfBoolean) => {
                            let column_with_suffix = format!("{}.{}", column, suffix);
                            if let Ok(exists) = hc
                                .get_exists_query_url(&dataset, &column_with_suffix, false)
                                .await
                            {
                                return ([("HX-Redirect", exists)], "").into_response();
                            }
                        }
                        _ => {
                            if let Ok(exists) =
                                hc.get_exists_query_url(&dataset, &column, false).await
                            {
                                return ([("HX-Redirect", exists)], "").into_response();
                            }
                        }
                    };
                }
            }
        }
    }
    "".into_response()
}

async fn tree_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Some(node) = state.db.get_node(&name) {
        node.clone()
    } else {
        state.db.clone()
    }
    .into_response()
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

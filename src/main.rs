mod data;
mod semconv;

use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Path, State},
    routing::get,
    Router,
};
use data::Node;
use semconv::{Attribute, Examples};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {}

#[derive(Template)]
#[template(path = "toplevel.html")]
struct TopLevelTemplate {
    level: String,
    level_parts: Vec<String>,
    level_links: Vec<String>,
    nodes: Vec<Node<Attribute>>,
}

#[derive(Clone)]
struct AppState {
    db: Node<Attribute>,
}

#[tokio::main]
async fn main() {
    // load our data
    let sc = semconv::SemanticConventions::new(&[
        "/Users/jerbly/Documents/code/public/semantic-conventions/model".to_owned(),
        "/Users/jerbly/Documents/code/eio-otel-semantic-conventions/model".to_owned(),
    ])
    .unwrap();
    let mut root = Node::new("root".to_string(), None);
    let mut keys: Vec<_> = sc.attribute_map.keys().collect();
    keys.sort();

    for k in keys {
        root.add_node(k, Some(sc.attribute_map[k].clone()));
    }
    let state = AppState { db: root };

    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/toplevel/:name", get(toplevel_handler))
        .with_state(state);

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> impl IntoResponse {
    IndexTemplate {}
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

async fn toplevel_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if name == "root" {
        return TopLevelTemplate {
            level: name.clone(),
            level_parts: vec![name.clone()],
            level_links: vec![name.clone()],
            nodes: state
                .db
                .children
                .values()
                .cloned()
                .collect::<Vec<Node<Attribute>>>(),
        };
    }
    if name.starts_with("root.") {
        let name = name.trim_start_matches("root.");
        if let Some(node) = state.db.get_node(name) {
            let level_parts = name.split('.').map(|s| s.to_owned()).collect();
            let level_links = get_links(&level_parts);
            return TopLevelTemplate {
                level: name.to_owned(),
                level_parts,
                level_links,
                nodes: node
                    .children
                    .values()
                    .cloned()
                    .collect::<Vec<Node<Attribute>>>(),
            };
        }
    }
    let level_parts = name.split('.').map(|s| s.to_owned()).collect();
    let level_links = get_links(&level_parts);
    if let Some(node) = state.db.get_node(&name) {
        TopLevelTemplate {
            level: name.clone(),
            level_parts,
            level_links,
            nodes: node
                .children
                .values()
                .cloned()
                .collect::<Vec<Node<Attribute>>>(),
        }
    } else {
        TopLevelTemplate {
            level: name.clone(),
            level_parts,
            level_links,
            nodes: vec![],
        }
    }
}

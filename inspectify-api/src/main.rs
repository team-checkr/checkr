use std::{collections::HashMap, convert::Infallible};

use axum::{
    extract::Path,
    response::{sse::Event, IntoResponse, Response, Sse},
    BoxError, Json, Router,
};
use ce_shell::Analysis;
use futures_util::{stream::BoxStream, Stream, StreamExt};
use rand::SeedableRng;
use tapi::{Endpoint, RouterExt};

#[derive(tapi::Tapi, serde::Serialize, serde::Deserialize)]
struct Person {
    name: String,
    friends: Vec<Person>,
    color: Color,
    words: HashMap<String, String>,
}

#[derive(tapi::Tapi, serde::Serialize, serde::Deserialize)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

// #[derive(tapi::Tapi, serde::Serialize, serde::Deserialize)]
// enum Stuff {
//     Hello,
//     World,
// }

#[tapi::tapi(path = "/", method = Get)]
async fn index() -> String {
    format!("hello, world!")
}

#[tapi::tapi(path = "/api", method = Get)]
async fn index_json(Json(p): Json<Person>) -> String {
    format!("hello, {}!", p.name)
}

#[derive(tapi::Tapi, serde::Serialize, serde::Deserialize)]
struct Msg {
    msg: String,
}
impl Msg {
    fn new(msg: String) -> Self {
        Self { msg }
    }
}

#[tapi::tapi(path = "/sse_example", method = Get)]
async fn sse_example() -> tapi::Sse<Msg> {
    tapi::Sse::new(futures_util::stream::iter(vec![
        Ok(Msg::new("Hello".to_string())),
        Ok(Msg::new("World".to_string())),
    ]))
}

#[derive(tapi::Tapi, serde::Serialize, serde::Deserialize)]
enum Test {
    A,
    B,
}

#[derive(tapi::Tapi, Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GenerateParams {
    analysis: Analysis,
}

#[tapi::tapi(path = "/generate", method = Post)]
async fn generate(Json(params): Json<GenerateParams>) -> Json<ce_shell::Input> {
    let input = params
        .analysis
        .gen_input(&mut rand::rngs::SmallRng::from_entropy());
    Json(input)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let endpoints = tapi::Endpoints::new([
        &index::endpoint as &dyn tapi::Endpoint,
        &index_json::endpoint,
        &sse_example::endpoint,
        &generate::endpoint,
    ])
    .with_ty::<ce_shell::Analysis>();

    let api = Router::new()
        .tapis(&endpoints)
        .layer(tower_http::cors::CorsLayer::permissive());
    let app = Router::new().nest("/api", api);

    let ts_client_path = std::path::PathBuf::from("./inspectify-app/src/lib/api.ts");
    // write TypeScript client if and only if the path already exists
    if ts_client_path.exists() {
        // only write if the contents are different
        let ts_client = endpoints.ts_client();
        let prev = std::fs::read_to_string(&ts_client_path).unwrap_or_default();
        if prev != ts_client {
            std::fs::write(&ts_client_path, ts_client).unwrap();
        }
    }

    println!("{}", endpoints.ts_client());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

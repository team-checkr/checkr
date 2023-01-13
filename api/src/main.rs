use std::net::SocketAddr;

use axum::{
    http::{header::CONTENT_TYPE, HeaderValue, Method, StatusCode},
    routing::{get, get_service, post},
    Router,
};
use tower_http::{cors::CorsLayer, services::ServeDir};

async fn hello() -> String {
    "Hello, world!".to_string()
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/hello", get(hello));
    // .fallback(
    //     get_service(ServeDir::new("ui/dist/")).handle_error(|error: std::io::Error| async move {
    //         (
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //             format!("Unhandled internal error: {}", error),
    //         )
    //     }),
    // );
    // NOTE: Use the files compiled into the binary
    // let app = app.fallback(static_dir.into_service());
    let app = app.layer(
        CorsLayer::new()
            .allow_origin("*".parse::<HeaderValue>().unwrap())
            .allow_headers([CONTENT_TYPE])
            .allow_methods([Method::GET, Method::POST]),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

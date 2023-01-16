use std::net::SocketAddr;

use axum::{
    http::{header::CONTENT_TYPE, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tower_http::cors::CorsLayer;

async fn hello() -> String {
    "Hello, world!".to_string()
}

#[axum::debug_handler]
async fn static_dir(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    static UI_DIR: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/../ui/dist/");

    if uri.path() == "/" {
        return Html(
            UI_DIR
                .get_file("index.html")
                .unwrap()
                .contents_utf8()
                .unwrap(),
        )
        .into_response();
    }

    match UI_DIR.get_file(&uri.path()[1..]) {
        Some(file) => {
            let mime_type = mime_guess::from_path(uri.path())
                .first_raw()
                .map(HeaderValue::from_static)
                .unwrap_or_else(|| {
                    HeaderValue::from_str(mime::APPLICATION_OCTET_STREAM.as_ref()).unwrap()
                });
            (
                [(axum::http::header::CONTENT_TYPE, mime_type)],
                file.contents(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
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
    let app = app.fallback(static_dir);
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

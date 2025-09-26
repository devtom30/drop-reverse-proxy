use axum::{Router, routing::get};
use axum::http::{HeaderMap, StatusCode};
use axum::http::response::Builder;
use axum::response::{IntoResponse, Response};

const TOKEN_NAME: &str = "dop_token";

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app()).await.unwrap();
}

fn app() -> Router {
    Router::new()
        .route("/tag", get(tag)) //.layer(tag_layer)
        .route("/play", get(play))//.layer(play_layer)
        .route("/{*path}", get(file))//.layer(file_layer)

}

async fn tag() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(TOKEN_NAME, "thetoken".parse().unwrap());
    (StatusCode::OK, headers)
}

async fn play() {}

async fn file() {}


#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        extract::connect_info::MockConnectInfo,
        http::{self, Request, StatusCode},
    };
    use http_body_util::{BodyExt, Empty}; // for `collect`
    use serde_json::{json, Value};
    use tokio::net::TcpListener;
    use tower::{Service, ServiceExt}; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn get_tag() {
        let app = app();

        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let response = app
            .oneshot(Request::builder().uri("/tag").body(Empty::new()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let header_map = response.headers();
        assert!(header_map.get(TOKEN_NAME).is_some());
        assert_eq!(header_map.get(TOKEN_NAME).unwrap(), "thetoken");
    }
}

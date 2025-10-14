use axum::{Router, routing::get, middleware, Error};
use axum::extract::{Path, Request};
use axum::http::{HeaderMap, StatusCode};
use axum::http::response::Builder;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use regex::Regex;
use serde::Serialize;

const TOKEN_NAME: &str = "dop_token";

const TAG_LIST: [&str; 3] = ["tag1", "tag2", "tag3"];

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app()).await.unwrap();
}

fn app() -> Router {
    Router::new()
        .route("/tag/{tag}", get(tag))
        .route("/play", get(play))//.layer(play_layer)
        .route("/{*path}", get(file))//.layer(file_layer)

}

#[derive(Debug)]
enum AppError {
    TagNotFound
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // How we want errors responses to be serialized
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        match &self {
            AppError::TagNotFound => {
                // While we could simply log the error here we would introduce
                // a side-effect to our conversion, instead add the AppError to
                // the Response as an Extension
                // Don't expose any details about the error to the client
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

async fn tag(
    Path(tag): Path<String>
) -> Result<(StatusCode, HeaderMap), AppError> {
    if let Some(tag_extracted) = extract_tag_from_path(tag.as_str()) {
        if check_tag(tag_extracted.as_str()) {
            let mut headers = HeaderMap::new();
            headers.insert(TOKEN_NAME, "thetoken".parse().unwrap());
            return Ok((StatusCode::OK, headers))
        }
    }

    Err(AppError::TagNotFound)
}

async fn tag_layer(
    path: Path<String>,
    request: Request,
    next: Next
) {

}

fn check_tag(tag: &str) -> bool {
    TAG_LIST.contains(&tag)
}

fn extract_tag_from_path(uri_path: &str) -> Option<String> {
    println!("match in {uri_path} ? ");
    let re = Regex::new(r"([^/]+)/?$").unwrap();
    if let Some(caps) = re.captures(uri_path) {
        let str = caps.get(1).unwrap().as_str().to_string();
        println!("match");
        Some(str)
    } else {
        println!("no match!");
        None
    }
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
            .oneshot(Request::builder().uri("/tag/tag1").body(Empty::new()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let header_map = response.headers();
        assert!(header_map.get(TOKEN_NAME).is_some());
        assert_eq!(header_map.get(TOKEN_NAME).unwrap(), "thetoken");
    }
}

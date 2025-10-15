use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{routing::get, Router};
use chrono::{NaiveDateTime};
use regex::Regex;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::ser::SerializeStruct;
use uuid::Uuid;

const TOKEN_NAME: &str = "dop_token";

const TAG_LIST: [&str; 3] = ["tag1", "tag2", "tag3"];

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let token_repo = InMemoryTokenRepo::default();
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
    };
    axum::serve(listener, app(app_state)).await.unwrap();
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/tag/{tag}", get(tag))
        .route("/play", get(play))//.layer(play_layer)
        .route("/{*path}", get(file))//.layer(file_layer)
        .with_state(state)
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

#[derive(Debug, Clone)]
struct Token {
    id: Uuid,
    create_date: NaiveDateTime,
    tag: String
}

impl Serialize for Token {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("Token", 3)?;
        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("create_date", &self.create_date.to_string())?;
        state.serialize_field("tag", &self.tag)?;
        state.end()
    }
}

#[derive(Clone)]
struct AppState {
    token_repo: Arc<dyn TokenRepo>,
}

async fn tag(
    State(state): State<AppState>,
    Path(tag): Path<String>
) -> Result<(StatusCode, HeaderMap), AppError> {
    if let Some(tag_extracted) = extract_tag_from_path(tag.as_str()) {
        if check_tag(tag_extracted.as_str()) {
            let uuid = Uuid::new_v4();

            let mut headers = HeaderMap::new();
            headers.insert(TOKEN_NAME, uuid.to_string().parse().unwrap());

            state.token_repo.save_token(&Token {
                id: uuid,
                create_date: NaiveDateTime::default(),
                tag: tag_extracted,
            });
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

fn check_token(
    path: Path<String>,
    request: Request,
    next: Next
) {
    request.headers();
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

trait TokenRepo: Send + Sync {
    fn get_token(&self, id: Uuid) -> Option<Token>;

    fn save_token(&self, token: &Token);
}

#[derive(Debug, Clone, Default)]
struct InMemoryTokenRepo {
    map: Arc<Mutex<HashMap<Uuid, Token>>>,
}

impl TokenRepo for InMemoryTokenRepo {
    fn get_token(&self, id: Uuid) -> Option<Token> {
        self.map.lock().unwrap().get(&id).cloned()
    }

    fn save_token(&self, token: &Token) {
        self.map.lock().unwrap().insert(token.id, token.clone());
    }
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;
    use axum::http::{Request, StatusCode};
    use http_body_util::Empty;
    use tower::ServiceExt;
    // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn get_tag() {
        let token_repo = InMemoryTokenRepo::default();
        let app_state = AppState {
            token_repo: Arc::new(token_repo.clone())
        };
        let app = app(app_state.clone());

        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let response = app
            .oneshot(Request::builder().uri("/tag/tag1").body(Empty::new()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let header_map = response.headers();
        assert!(header_map.get(TOKEN_NAME).is_some());

        let token = app_state.token_repo.get_token(Uuid::from_str(header_map.get(TOKEN_NAME).unwrap().to_str().unwrap()).unwrap());
        assert!(token.is_some());
    }

    #[tokio::test]
    async fn get_tag_error() {
        let token_repo = InMemoryTokenRepo::default();
        let app_state = AppState {
            token_repo: Arc::new(token_repo.clone())
        };
        let app = app(app_state);

        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let response = app
            .oneshot(Request::builder().uri("/tag/tagUnknown").body(Empty::new()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}

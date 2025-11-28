use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use chrono::NaiveDateTime;
use regex::Regex;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use redis::Commands;

pub const TOKEN_NAME: &str = "dop_token";

const TAG_LIST: [&str; 3] = ["tag1", "tag2", "tag3"];

pub fn app(state: AppState) -> Router {
    Router::new()
        .route(
            "/tag/{tag}",
            get(tag).route_layer(axum::middleware::from_fn(tag_guard)),
        )
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
        match &self {
            AppError::TagNotFound => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
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
pub struct AppState {
    pub token_repo: Arc<dyn TokenRepo>,
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

// Route guard for /tag that validates the requested tag is allowed
async fn tag_guard(req: Request, next: Next) -> Response {
    let path = req.uri().path();
    if let Some(tag) = extract_tag_from_path(path) {
        if check_tag(tag.as_str()) {
            return next.run(req).await;
        }
    }
    AppError::TagNotFound.into_response()
}

// Placeholder for future token checks
fn check_token(_path: Path<String>, _request: Request, _next: Next) {}

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

pub trait TokenRepo: Send + Sync {
    fn get_token(&self, id: Uuid) -> Option<Token>;

    fn save_token(&self, token: &Token);
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryTokenRepo {
    map: Arc<Mutex<HashMap<Uuid, Token>>>,
}

#[derive(Debug, Clone)]
pub struct TokenRepoDB {
    client: redis::Client,
}

impl Default for TokenRepoDB {
    fn default() -> Self {
        let client = redis::Client::open("redis://127.0.0.1/")
            .expect("failed to create a redis client");
        Self { client }
    }
}

impl TokenRepoDB {
    pub fn new(redis_url: &str) -> redis::RedisResult<Self> {
        Ok(Self { client: redis::Client::open(redis_url)? })
    }
}

impl TokenRepo for InMemoryTokenRepo {
    fn get_token(&self, id: Uuid) -> Option<Token> {
        self.map.lock().unwrap().get(&id).cloned()
    }

    fn save_token(&self, token: &Token) {
        self.map.lock().unwrap().insert(token.id, token.clone());
    }
}

impl TokenRepo for TokenRepoDB {
    fn get_token(&self, id: Uuid) -> Option<Token> {
        let mut conn = match self.client.get_connection() {
            Ok(c) => c,
            Err(_) => return None,
        };
        let key = format!("token:{}", id);
        let id_s: Option<String> = conn.hget(&key, "id").ok();
        let tag: Option<String> = conn.hget(&key, "tag").ok();
        let create_date_s: Option<String> = conn.hget(&key, "create_date").ok();

        match (id_s, tag, create_date_s) {
            (Some(id_str), Some(tag), Some(cd_str)) => {
                if id_str != id.to_string() {
                    return None;
                }
                let create_date = chrono::NaiveDateTime::parse_from_str(&cd_str, "%Y-%m-%d %H:%M:%S").ok()?;
                Some(Token { id, create_date, tag })
            }
            _ => None,
        }
    }

    fn save_token(&self, token: &Token) {
        if let Ok(mut conn) = self.client.get_connection() {
            let key = format!("token:{}", token.id);
            let _: redis::RedisResult<()> = conn.hset_multiple(
                &key,
                &[
                    ("id", token.id.to_string()),
                    ("create_date", token.create_date.format("%Y-%m-%d %H:%M:%S").to_string()),
                    ("tag", token.tag.clone()),
                ],
            );
        }
    }
}

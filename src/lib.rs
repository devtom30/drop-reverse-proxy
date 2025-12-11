use axum::extract::{ConnectInfo, Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use chrono::NaiveDateTime;
use derive_new::new;
use redis::Commands;
use regex::Regex;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub const TOKEN_NAME: &str = "dop_token";
pub const MAX_BAD_ATTEMPTS: u32 = 10;

pub fn app(state: AppState) -> Router {
    Router::new()
        .route(
            "/tag/{tag}",
            get(tag).route_layer(axum::middleware::from_fn_with_state(state.clone(), tag_guard)),
        )
        .route(
            "/play",
            get(play).route_layer(axum::middleware::from_fn_with_state(state.clone(), token_guard))
        )
        .route(
            "/{*path}",
            get(file).route_layer(axum::middleware::from_fn_with_state(state.clone(), token_guard))
        )
        .route(
            "/",
            get(|| async { Ok::<_, axum::http::StatusCode>(StatusCode::UNAUTHORIZED) })
        )
        .with_state(state)
}

#[derive(Debug)]
enum AppError {
    TagNotFound,
    Unauthorized,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // How we want errors responses to be serialized
        match &self {
            AppError::TagNotFound => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            AppError::Unauthorized => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

#[derive(Debug, Clone, new)]
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
    pub tag_repo: Arc<dyn TagRepo>,
    pub ip_repo: Arc<dyn IpRepo>
}

async fn tag(
    State(state): State<AppState>,
    Path(tag): Path<String>,
) -> Result<(StatusCode, HeaderMap), AppError> {
    if let Some(tag_extracted) = extract_tag_from_path(tag.as_str()) {
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

    Err(AppError::TagNotFound)
}

// Route guard for /tag that validates the requested tag is allowed
async fn tag_guard(
    State(state): State<AppState>,
    ConnectInfo(connect_info): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next
) -> Response {
    println!("connect info ip {:#?}", connect_info.ip());
    if !check_ip(connect_info.ip(), state.ip_repo) {
        return AppError::Unauthorized.into_response();
    }
    let path = req.uri().path();
    if let Some(tag) = extract_tag_from_path(path) {
        if check_tag(tag.as_str(), state.tag_repo) {
            return next.run(req).await;
        }
    }

    AppError::TagNotFound.into_response()
}

// Placeholder for future token checks
async fn token_guard(
    State(state): State<AppState>,
    ConnectInfo(connect_info): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next
) -> Response {
    if !check_ip(connect_info.ip(), state.ip_repo) {
        return AppError::Unauthorized.into_response();
    }
    let headers = req.headers().clone();
    if let Some(header_token) = headers.get(TOKEN_NAME) {
        if let Ok(header_token_str) = header_token.to_str() {
            if let Ok(token_uuid_requested) = Uuid::parse_str(header_token_str) {
                if let Some(_token) = state.token_repo.get_token(token_uuid_requested) {
                    return next.run(req).await;
                }
            }
        }
    }
    AppError::Unauthorized.into_response()
}

fn check_tag(tag: &str, tag_repo: Arc<dyn TagRepo>) -> bool {
    tag_repo.get(tag.to_string()).is_some()
}

fn check_ip(ip_addr: IpAddr, ip_repo: Arc<dyn IpRepo>) -> bool {
    match ip_repo.get(&ip_addr) {
        Some(ip) => {
            ip_repo.save_or_update(Ip {
                addr: ip_addr,
                first_seen: ip.first_seen,
                last_seen: chrono::NaiveDateTime::default(),
                nb_bad_attempts: ip.nb_bad_attempts + 1,
            });
            ip.nb_bad_attempts < MAX_BAD_ATTEMPTS
        },
        None => {
            ip_repo.save_or_update(Ip {
                addr: ip_addr,
                first_seen: chrono::NaiveDateTime::default(),
                last_seen: chrono::NaiveDateTime::default(),
                nb_bad_attempts: 1,
            });
            true
        },
    }
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

async fn play() -> Result<StatusCode, AppError> {
    Ok(StatusCode::OK)
}

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

pub trait TagRepo: Send + Sync {
    fn get(&self, tag: String) -> Option<Tag>;

    fn save(&self, tag: &Tag);
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryTagRepo {
    map: Arc<Mutex<HashMap<String, Tag>>>,
}

impl TagRepo for InMemoryTagRepo {
    fn get(&self, name: String) -> Option<Tag> {
        self.map.lock().unwrap().get(&name).cloned()
    }

    fn save(&self, tag: &Tag) {
        self.map.lock().unwrap().insert(tag.id.clone(), tag.clone());
    }
}

#[derive(Debug, Clone)]
pub struct TagRepoDB {
    client: redis::Client,
}

impl Default for TagRepoDB {
    fn default() -> Self {
        let client = create_redis_client();
        Self { client }
    }
}

fn create_redis_client() -> redis::Client {
    redis::Client::open("redis://127.0.0.1/")
        .expect("failed to create a redis client")
}

impl TagRepoDB {
    pub fn new(redis_url: &str) -> redis::RedisResult<Self> {
        Ok(Self { client: redis::Client::open(redis_url)? })
    }
}

#[derive(Debug, Clone, new)]
pub struct Tag {
    id: String,
    create_date: NaiveDateTime,
}

impl TagRepo for TagRepoDB {
    fn get(&self, tag: String) -> Option<Tag> {
        let mut conn = match self.client.get_connection() {
            Ok(c) => c,
            Err(_) => return None,
        };
        let key = format!("tag:{}", tag);
        let create_date_s: Option<String> = conn.hget(&key, "create_date").ok();

        match create_date_s {
            Some(cd_str) => {
                let create_date = chrono::NaiveDateTime::parse_from_str(&cd_str, "%Y-%m-%d %H:%M:%S").ok()?;
                Some(Tag {
                    id: tag,
                    create_date,
                })
            }
            _ => None,
        }
    }

    fn save(&self, tag: &Tag) {
        if let Ok(mut conn) = self.client.get_connection() {
            let key = format!("tag:{}", tag.id.clone());
            let _: redis::RedisResult<()> = conn.hset_multiple(
                &key,
                &[
                    ("id", tag.id.clone()),
                    ("create_date", tag.create_date.format("%Y-%m-%d %H:%M:%S").to_string())
                ],
            );
        }
    }
}

#[derive(Debug, Clone, new)]
pub struct Ip {
    addr: IpAddr,
    first_seen: NaiveDateTime,
    last_seen: NaiveDateTime,
    nb_bad_attempts: u32,
}

#[derive(Debug, Clone)]
pub struct IpRepoDB {
    client: redis::Client,
}

impl Default for IpRepoDB {
    fn default() -> Self {
        let client = create_redis_client();
        Self { client }
    }
}

impl IpRepoDB {
    pub fn new(redis_url: &str) -> redis::RedisResult<Self> {
        Ok(Self { client: redis::Client::open(redis_url)? })
    }
}

pub trait IpRepo: Send + Sync {
    fn get(&self, ip_addr: &IpAddr) -> Option<Ip>;
    fn save_or_update(&self, ip: Ip);
}

impl IpRepo for IpRepoDB {
    fn get(&self, ip_addr: &IpAddr) -> Option<Ip> {
        let mut conn = match self.client.get_connection() {
            Ok(c) => c,
            Err(_) => return None,
        };
        let key = format!("ip:{}", ip_addr.to_string());
        let ip_addr_s: Option<String> = conn.hget(&key, "addr").ok();
        let first_seen: Option<String> = conn.hget(&key, "first_seen").ok();
        let last_seen: Option<String> = conn.hget(&key, "last_seen").ok();
        let nb_bad_attempts: Option<String> = conn.hget(&key, "nb_bad_attempts").ok();

        match (ip_addr_s, first_seen, last_seen, nb_bad_attempts) {
            (Some(_ip_addr_str), Some(fs_str), Some(ls_str), Some(nb_bad_attempts_str)) => {
                Some(Ip {
                    addr: ip_addr.clone(),
                    first_seen: chrono::NaiveDateTime::parse_from_str(&fs_str, "%Y-%m-%d %H:%M:%S").ok()?,
                    last_seen: chrono::NaiveDateTime::parse_from_str(&ls_str, "%Y-%m-%d %H:%M:%S").ok()?,
                    nb_bad_attempts: nb_bad_attempts_str.parse::<u32>().ok()?,
                })
            }
            _ => None,
        }

    }

    fn save_or_update(&self, ip: Ip) {
        if let Ok(mut conn) = self.client.get_connection() {
            let key = format!("ip:{}", ip.addr.to_string());
            let _: redis::RedisResult<()> = conn.hset_multiple(
                &key,
                &[
                    (
                        "addr",
                        ip.addr.to_string()
                    ),
                    (
                        "first_seen",
                        ip.first_seen.format("%Y-%m-%d %H:%M:%S").to_string()
                    ),
                    (
                        "last_seen",
                        ip.last_seen.format("%Y-%m-%d %H:%M:%S").to_string()
                    ),
                    (
                        "nb_bad_attempts",
                        ip.nb_bad_attempts.to_string()
                    )
                ],
            );
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryIpRepo {
    map: Arc<Mutex<HashMap<IpAddr, Ip>>>,
}

impl IpRepo for InMemoryIpRepo {
    fn get(&self, ip_addr: &IpAddr) -> Option<Ip> {
        self.map.lock().unwrap().get(ip_addr).cloned()
    }

    fn save_or_update(&self, ip: Ip) {
        self.map.lock().expect("can't lock mutex").insert(ip.addr.clone(), ip);
    }
}

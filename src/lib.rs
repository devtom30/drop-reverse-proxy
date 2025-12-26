use axum::extract::{ConnectInfo, Path, Request, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use chrono::NaiveDateTime;
use derive_new::new;
use redis::Commands;
use regex::Regex;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use figment::Figment;
use figment::providers::{Format, Toml};
use uuid::Uuid;

pub const TOKEN_NAME: &str = "dop_token";

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
            get(|| async { Ok::<_, StatusCode>(StatusCode::UNAUTHORIZED) })
        )
        // check route ""
        .with_state(state)
}

#[derive(Debug)]
enum AppError {
    TagNotFound,
    Unauthorized,
    InternalError
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // How we want errors responses to be serialized
        match &self {
            AppError::TagNotFound => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            AppError::Unauthorized => StatusCode::UNAUTHORIZED.into_response(),
            AppError::InternalError => StatusCode::INTERNAL_SERVER_ERROR.into_response()
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
    pub ip_repo: Arc<dyn IpRepo>,
    pub conf: Conf,
}

async fn tag(
    State(state): State<AppState>,
    ConnectInfo(connect_info): ConnectInfo<SocketAddr>,
    Path(tag): Path<String>,
) -> Result<Response, AppError> {
    if let Some(tag_extracted) = extract_tag_from_path(tag.as_str()) {
        let uuid = Uuid::new_v4();

        state.token_repo.save_token(&Token {
            id: uuid,
            create_date: NaiveDateTime::default(),
            tag: tag_extracted.clone(),
        });

        let mut uri_new = String::from(state.conf.redirect_uri);
        uri_new.push_str("/tag/");
        uri_new.push_str(&tag_extracted);
        uri_new.push_str("/index.html");
        println!("calling url {uri_new}");
        return match reqwest::get(uri_new).await {
            Ok(resp) => {
                let mut response = resp.bytes().await.unwrap().into_response().into_body().into_response();
                let header_value_str = format!("{}={}", TOKEN_NAME, uuid);
                match HeaderValue::from_str(header_value_str.as_str()) {
                    Ok(header_value) => {
                        response.headers_mut().append(
                            SET_COOKIE,
                            header_value
                        );
                        Ok(response)
                    }
                    Err(_) => {
                        Err(AppError::InternalError)
                    }
                }
            },
            Err(_) => {
                increment_ip_nb_bad_attempts(&connect_info.ip(), &state.ip_repo);
                Err(AppError::TagNotFound)
            },
        }
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
    if !check_ip(connect_info.ip(), &state.ip_repo, state.conf.max_attempts) {
        increment_ip_nb_bad_attempts(&connect_info.ip(), &state.ip_repo);
        return AppError::Unauthorized.into_response();
    }
    let path = req.uri().path();
    if let Some(tag) = extract_tag_from_path(path) {
        if check_tag(tag.as_str(), state.tag_repo) {
            state.ip_repo.save_or_update(&connect_info.ip(), 0);
            return next.run(req).await;
        } else {
            increment_ip_nb_bad_attempts(&connect_info.ip(), &state.ip_repo);
        }
    }

    AppError::TagNotFound.into_response()
}

fn increment_ip_nb_bad_attempts(ip_addr: &IpAddr, ip_repo: &Arc<dyn IpRepo>) {
    match ip_repo.get(&ip_addr) {
        None => {}
        Some(ip) => {
            ip_repo.save_or_update(ip_addr, ip.nb_bad_attempts + 1);
        }
    }
}

// Placeholder for future token checks
async fn token_guard(
    State(state): State<AppState>,
    ConnectInfo(connect_info): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next
) -> Response {
    if !check_ip(connect_info.ip(), &state.ip_repo, state.conf.max_attempts) {
        increment_ip_nb_bad_attempts(&connect_info.ip(), &state.ip_repo);
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
    increment_ip_nb_bad_attempts(&connect_info.ip(), &state.ip_repo);
    AppError::Unauthorized.into_response()
}

fn check_tag(tag: &str, tag_repo: Arc<dyn TagRepo>) -> bool {
    tag_repo.get(tag.to_string()).is_some()
}

fn check_ip(ip_addr: IpAddr, ip_repo: &Arc<dyn IpRepo>, max_bad_attempts: u8) -> bool {
    match ip_repo.get(&ip_addr) {
        Some(ip) => {
            println!("{:#?}", ip);
            ip.nb_bad_attempts < max_bad_attempts as u32
        },
        None => true,
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

async fn play(
    State(state): State<AppState>,
    ConnectInfo(connect_info): ConnectInfo<SocketAddr>,
    req: Request,
) -> Result<Response, AppError> {
    let headers = req.headers().clone();
    if let Some(header_token) = headers.get(TOKEN_NAME) {
        if let Ok(token_str) = header_token.to_str() {
            if let Ok(token_uuid_requested) = Uuid::parse_str(token_str) {
                let token_opt = state.token_repo.get_token(token_uuid_requested);
                if let Some(token) = token_opt {
                    let mut uri_new = String::from(state.conf.redirect_uri);
                    uri_new.push_str("/tag/");
                    uri_new.push_str(&token.tag);
                    uri_new.push_str("/playlist.m3u8");
                    println!("calling {uri_new}");
                    return match reqwest::get(uri_new).await {
                        Ok(resp) => {
                            Ok(resp.bytes().await.unwrap().into_response())
                        },
                        Err(_) => {
                            increment_ip_nb_bad_attempts(&connect_info.ip(), &state.ip_repo);
                            Err(AppError::TagNotFound)
                        },
                    }
                }
            }
        }
    }
    Ok(StatusCode::UNAUTHORIZED.into_response())
}

async fn file(
    State(state): State<AppState>,
    ConnectInfo(connect_info): ConnectInfo<SocketAddr>,
    Path(path): Path<String>,
    req: Request,
) -> Result<Response, AppError> {
    let headers = req.headers().clone();
    if let Some(header_token) = headers.get(TOKEN_NAME) {
        if let Ok(token_str) = header_token.to_str() {
            if let Ok(token_uuid_requested) = Uuid::parse_str(token_str) {
                let token_opt = state.token_repo.get_token(token_uuid_requested);
                if let Some(token) = token_opt {
                    let mut uri_new = String::from(state.conf.redirect_uri);
                    uri_new.push_str("/tag/");
                    uri_new.push_str(&token.tag);
                    uri_new.push('/');
                    uri_new.push_str(path.as_str());

                    println!("calling {uri_new}");
                    return match reqwest::get(uri_new).await {
                        Ok(resp) => {
                            resp.headers().iter().for_each(|(header_name, header_value)| {
                                println!("header: {:#?} - {:#?}", header_name, header_value);
                            });
                            Ok(resp.bytes().await.unwrap().into_response().into_body().into_response())
                        },
                        Err(_) => {
                            increment_ip_nb_bad_attempts(&connect_info.ip(), &state.ip_repo);
                            Err(AppError::TagNotFound)
                        },
                    }
                }
            }
        }
    }
    Ok(StatusCode::UNAUTHORIZED.into_response())
}

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
                let create_date = NaiveDateTime::parse_from_str(&cd_str, "%Y-%m-%d %H:%M:%S").ok()?;
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
                let create_date = NaiveDateTime::parse_from_str(&cd_str, "%Y-%m-%d %H:%M:%S").ok()?;
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

impl Ip {
    pub fn addr(&self) -> &IpAddr {
        &self.addr
    }
    pub fn first_seen(&self) -> &NaiveDateTime {
        &self.first_seen
    }
    pub fn last_seen(&self) -> &NaiveDateTime {
        &self.last_seen
    }
    pub fn nb_bad_attempts(&self) -> &u32 {
        &self.nb_bad_attempts
    }
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
    fn save_or_update(&self, ip_addr: &IpAddr, nb_bad_attempts: u32);
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
                    first_seen: NaiveDateTime::parse_from_str(&fs_str, "%Y-%m-%d %H:%M:%S").ok()?,
                    last_seen: NaiveDateTime::parse_from_str(&ls_str, "%Y-%m-%d %H:%M:%S").ok()?,
                    nb_bad_attempts: nb_bad_attempts_str.parse::<u32>().ok()?,
                })
            }
            _ => None,
        }

    }

    fn save_or_update(&self, ip_addr: &IpAddr, nb_bad_attempts: u32) {
        if let Ok(mut conn) = self.client.get_connection() {
            let mut first_seen = NaiveDateTime::default();
            let last_seen= first_seen;
            if let Some(ip) = self.get(ip_addr) {
                first_seen = ip.first_seen;
            }
            let key = format!("ip:{}", ip_addr.to_string());
            let _: redis::RedisResult<()> = conn.hset_multiple(
                &key,
                &[
                    (
                        "addr",
                        ip_addr.to_string()
                    ),
                    (
                        "first_seen",
                        first_seen.format("%Y-%m-%d %H:%M:%S").to_string()
                    ),
                    (
                        "last_seen",
                        last_seen.format("%Y-%m-%d %H:%M:%S").to_string()
                    ),
                    (
                        "nb_bad_attempts",
                        nb_bad_attempts.to_string()
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

    fn save_or_update(&self, ip_addr: &IpAddr, nb_bad_attempts: u32) {
        let mut first_seen = NaiveDateTime::default();
        let last_seen= first_seen;
        if let Some(ip) = self.get(ip_addr) {
            first_seen = ip.first_seen;
        }
        self.map.lock().expect("can't lock mutex").insert(
            ip_addr.clone(),
            Ip::new(
                *ip_addr,
                first_seen,
                last_seen,
                nb_bad_attempts
            ));

    }
}

#[derive(Clone, Deserialize)]
pub struct Conf {
    redirect_uri: String,
    bind_addr: String,
    max_attempts: u8,
    tags: Vec<String>
}

impl Conf {

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    pub fn bind_addr(&self) -> &str {
        &self.bind_addr
    }

    pub fn max_attempts(&self) -> u8 {
        self.max_attempts
    }

    pub fn new(redirect_uri: String, bind_addr: String, max_attempts: u8, tags: Vec<String>) -> Self {
        Self { redirect_uri, bind_addr, max_attempts, tags }
    }

    pub fn tags(&self) -> &Vec<String> {
        &self.tags
    }
}

pub fn create_conf_from_toml_file(relative_path: &str) -> figment::Result<Conf> {
    Figment::new()
        .merge(Toml::file(relative_path))
        .extract()
}

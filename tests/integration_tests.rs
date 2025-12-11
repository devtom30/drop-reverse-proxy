use std::net::{IpAddr, SocketAddr};
use axum::http::{Request, StatusCode};
use chrono::NaiveDateTime;
use drop_reverse_proxy::{app, AppState, InMemoryIpRepo, InMemoryTagRepo, InMemoryTokenRepo, Ip, IpRepo, IpRepoDB, Tag, TagRepo, TagRepoDB, Token, TokenRepo, TokenRepoDB, TOKEN_NAME};
use http_body_util::Empty;
use std::process::Command;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use axum::extract::ConnectInfo;
use tower::ServiceExt;
use uuid::Uuid;

fn init_in_memory_tag_repo() -> InMemoryTagRepo {
    let tag_repo = InMemoryTagRepo::default();
    ["tag1", "tag2", "tag3"].iter()
        .for_each(|t| tag_repo.save(&Tag::new(t.to_string(), NaiveDateTime::default())));
    tag_repo
}

fn init_redis_tag_repo(redis_url: &String) -> Result<TagRepoDB, redis::RedisError> {
    let tag_repo_db = TagRepoDB::new(redis_url)?;
    tag_repo_db.save(&Tag::new("tag1".to_string(), NaiveDateTime::default()));
    tag_repo_db.save(&Tag::new("tag2".to_string(), NaiveDateTime::default()));
    tag_repo_db.save(&Tag::new("tag3".to_string(), NaiveDateTime::default()));
    Ok(tag_repo_db)
}

#[tokio::test]
async fn get_tag() {
    let token_repo = InMemoryTokenRepo::default();
    let tag_repo = init_in_memory_tag_repo();
    let ip_repo = IpRepoDB::default();
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo.clone()),
    };
    let app = app(app_state.clone());

    // `Router` implements `tower::Service<Request<Body>>` so we can
    // call it like any tower service, no need to run an HTTP server

    let mut req = Request::builder()
        .uri("/tag/tag1")
        .body(Empty::new())
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127,0,0,1], 12345))));
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let header_map = response.headers();
    assert!(header_map.get(TOKEN_NAME).is_some());

    let token = app_state.token_repo.get_token(Uuid::from_str(header_map.get(TOKEN_NAME).unwrap().to_str().unwrap()).unwrap());
    assert!(token.is_some());
}

#[tokio::test]
async fn get_tag_error() {
    let token_repo = InMemoryTokenRepo::default();
    let tag_repo = InMemoryTagRepo::default();
    let ip_repo = IpRepoDB::default();
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo),
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

#[tokio::test]
async fn tag_not_in_list_returns_500_and_no_token_header() {
    // Arrange: use in-memory repo
    let token_repo = InMemoryTokenRepo::default();
    let tag_repo = InMemoryTagRepo::default();
    let ip_repo = IpRepoDB::default();
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo),
    };
    let app = app(app_state);

    // Act: request a tag that is not in the allowed list
    let response = app
        .oneshot(
            Request::builder()
                .uri("/tag/not-allowed-tag")
                .body(Empty::new())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: 500 returned by the guard/handler and no token header set
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.headers().get(TOKEN_NAME).is_none());
}

#[tokio::test]
async fn save_and_get_token_from_repo() {
    // Arrange: app with in-memory repo
    let token_repo = InMemoryTokenRepo::default();
    let tag_repo = init_in_memory_tag_repo();
    let ip_repo = IpRepoDB::default();
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo),
    };
    let app = app(app_state.clone());

    // Act: call the endpoint that saves a token with tag2
    let mut req = Request::builder()
        .uri("/tag/tag2")
        .body(Empty::new())
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127,0,0,1], 12345))));
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Extract token id from header
    let headers = response.headers();
    let token_id_header = headers.get(TOKEN_NAME).expect("token header missing");
    let token_id = Uuid::from_str(token_id_header.to_str().unwrap()).unwrap();

    // Assert: repo contains the saved token and fields match
    let token = app_state.token_repo.get_token(token_id).expect("token not found in repo");

    // Serialize to inspect private fields
    let json = serde_json::to_value(&token).unwrap();
    assert_eq!(json.get("id").and_then(|v| v.as_str()), Some(token_id.to_string().as_str()));
    assert_eq!(json.get("tag").and_then(|v| v.as_str()), Some("tag2"));
}

// Ensure the container is stopped when the test ends
struct DockerGuard(String);
impl Drop for DockerGuard {
    fn drop(&mut self) {
        println!("docker stop redis container");
        let _ = Command::new("docker").args(["stop", &self.0]).output();
    }
}

fn init_redis_container() -> Option<(DockerGuard, String)> {
    // Try to launch a Redis container using Docker. If Docker is unavailable, skip the test.
    // 1) Ensure docker is available
    if Command::new("docker").arg("--version").output().is_err() {
        eprintln!("Skipping Redis-backed test: Docker CLI not available");
        return None;
    }

    // 2) Run a disposable Redis container with published random port
    let run_out = match Command::new("docker")
        .args(["run", "-d", "-P", "--rm", "redis:7-alpine"]) // lightweight image
        .output()
    {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!("Skipping Redis-backed test: docker run failed: {}", String::from_utf8_lossy(&o.stderr));
            return None;
        }
        Err(e) => {
            eprintln!("Skipping Redis-backed test: cannot run docker: {e}");
            return None;
        }
    };
    let container_id = String::from_utf8_lossy(&run_out.stdout).trim().to_string();

    let guard = DockerGuard(container_id.clone());

    // 3) Obtain the published host port for Redis (container port 6379)
    let port_out = match Command::new("docker").args(["port", &container_id, "6379/tcp"]).output() {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!("Skipping Redis-backed test: docker port failed: {}", String::from_utf8_lossy(&o.stderr));
            return None;
        }
        Err(e) => {
            eprintln!("Skipping Redis-backed test: cannot get docker port: {e}");
            return None;
        }
    };
    let port_stdout = String::from_utf8_lossy(&port_out.stdout);
    // docker may print multiple lines (IPv4 and IPv6). Take first non-empty line and parse last ':'
    let host_port = match port_stdout
        .lines()
        .find(|l| !l.trim().is_empty())
        .and_then(|l| l.rsplit(':').next())
        .and_then(|p| p.trim().parse::<u16>().ok())
    {
        Some(p) => p,
        None => {
            eprintln!("Skipping Redis-backed test: unable to parse published port from '{}':", port_stdout);
            return None;
        }
    };
    let redis_url = format!("redis://127.0.0.1:{}/", host_port);

    // 4) Wait for Redis to be ready (retry for a short period)
    let mut ready = false;
    for _ in 0..30 { // ~3 seconds
        if let Ok(client) = redis::Client::open(redis_url.as_str()) {
            if client.get_connection().is_ok() {
                ready = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    if !ready {
        eprintln!("Skipping Redis-backed test: Redis in container not ready on {}", redis_url);
    } else {
        return Some((guard, redis_url));
    }
    None
}

#[tokio::test]
async fn save_and_get_token_from_db() {
    let (_docker_guard, redis_url) = init_redis_container().unwrap();

    // Arrange: app with Redis-backed repo pointing to the container
    let token_repo = TokenRepoDB::new(&redis_url).expect("failed to create TokenRepoDB");
    let tag_repo = init_redis_tag_repo(&redis_url).expect("failed to init TagRepoDB");
    let ip_repo = IpRepoDB::new(&redis_url).expect("failed to create IpRepoDB");
    ip_repo.save_or_update(Ip::new(IpAddr::from([127,0,0,1]), NaiveDateTime::default(), NaiveDateTime::default(), 0));
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo),
    };
    let app = app(app_state.clone());

    // Act: call the endpoint that saves a token with tag2
let mut req = Request::builder()
        .uri("/tag/tag2")
        .body(Empty::new())
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127,0,0,1], 12345))));
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Extract token id from header
    let headers = response.headers();
    let token_id_header = headers.get(TOKEN_NAME).expect("token header missing");
    let token_id = Uuid::from_str(token_id_header.to_str().unwrap()).unwrap();

    // Assert: repo contains the saved token and fields match
    let token = app_state
        .token_repo
        .get_token(token_id)
        .expect("token not found in db repo");

    // Serialize to inspect private fields
    let json = serde_json::to_value(&token).unwrap();
    assert_eq!(json.get("id").and_then(|v| v.as_str()), Some(token_id.to_string().as_str()));
    assert_eq!(json.get("tag").and_then(|v| v.as_str()), Some("tag2"));
}

#[tokio::test]
async fn get_play_is_authorized_token() {
    let token_repo = InMemoryTokenRepo::default();
    let tag_repo = InMemoryTagRepo::default();
    let ip_repo = InMemoryIpRepo::default();
    let token_uuid_valid = Uuid::new_v4();
    let tag_ok = "tag1";
    let token = Token::new(
        token_uuid_valid,
        NaiveDateTime::default(),
        tag_ok.to_string()
    );
    token_repo.save_token(&token);
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo),
    };
    let app = app(app_state.clone());

    // `Router` implements `tower::Service<Request<Body>>` so we can
    // call it like any tower service, no need to run an HTTP server.
    let mut req = Request::builder()
        .header(TOKEN_NAME, token_uuid_valid.to_string())
        .uri("/play")
        .body(Empty::new())
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127,0,0,1], 12345))));
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_play_is_not_authorized_token() {
    let token_repo = InMemoryTokenRepo::default();
    let tag_repo = InMemoryTagRepo::default();
    let ip_repo = InMemoryIpRepo::default();
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo),
    };
    let app = app(app_state.clone());

    // `Router` implements `tower::Service<Request<Body>>` so we can
    // call it like any tower service, no need to run an HTTP server.
    let mut req = Request::builder()
        .header(TOKEN_NAME, "dummy token")
        .uri("/play")
        .body(Empty::new())
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127,0,0,1], 12345))));
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_play_is_not_authorized_token_when_random_path_and_no_token_header() {
    let token_repo = InMemoryTokenRepo::default();
    let tag_repo = init_in_memory_tag_repo();
    let ip_repo = InMemoryIpRepo::default();
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo),
    };
    let app = app(app_state.clone());

    for path in [
        "/",
        "/random",
        "/random/path",
        "/random/path?query=param",
        "/random/path#fragment"] {
        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let mut req = Request::builder()
            .uri(path)
            .body(Empty::new())
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127,0,0,1], 12345))));
        let response = app.clone().oneshot(req).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn get_play_is_authorized_token_and_ip_is_allowed() {
    let token_repo = InMemoryTokenRepo::default();
    let tag_repo = InMemoryTagRepo::default();
    let ip_repo = InMemoryIpRepo::default();
    let ip_addr = [127,0,0,1];
    ip_repo.save_or_update(Ip::new(IpAddr::from(ip_addr), NaiveDateTime::default(), NaiveDateTime::default(), 5));
    let token_uuid_valid = Uuid::new_v4();
    let tag_ok = "tag1";
    let token = Token::new(
        token_uuid_valid,
        NaiveDateTime::default(),
        tag_ok.to_string()
    );
    token_repo.save_token(&token);
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo),
    };
    let app = app(app_state.clone());

    // `Router` implements `tower::Service<Request<Body>>` so we can
    // call it like any tower service, no need to run an HTTP server.
    let mut req = Request::builder()
        .header(TOKEN_NAME, token_uuid_valid.to_string())
        .uri("/play")
        .body(Empty::new())
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127,0,0,1], 12345))));
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_play_is_authorized_token_and_ip_is_not_allowed() {
    let token_repo = InMemoryTokenRepo::default();
    let tag_repo = InMemoryTagRepo::default();
    let ip_repo = InMemoryIpRepo::default();
    let ip_addr = [127,0,0,1];
    ip_repo.save_or_update(Ip::new(IpAddr::from(ip_addr), NaiveDateTime::default(), NaiveDateTime::default(), 10));
    let token_uuid_valid = Uuid::new_v4();
    let tag_ok = "tag1";
    let token = Token::new(
        token_uuid_valid,
        NaiveDateTime::default(),
        tag_ok.to_string()
    );
    token_repo.save_token(&token);
    let app_state = AppState {
        token_repo: Arc::new(token_repo.clone()),
        tag_repo: Arc::new(tag_repo.clone()),
        ip_repo: Arc::new(ip_repo),
    };
    let app = app(app_state.clone());

    // `Router` implements `tower::Service<Request<Body>>` so we can
    // call it like any tower service, no need to run an HTTP server.
    let mut req = Request::builder()
        .header(TOKEN_NAME, token_uuid_valid.to_string())
        .uri("/play")
        .body(Empty::new())
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from((ip_addr, 12345))));
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
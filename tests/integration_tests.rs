use axum::http::{Request, StatusCode};
use drop_reverse_proxy::{app, AppState, InMemoryTokenRepo, TOKEN_NAME};
use http_body_util::Empty;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

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
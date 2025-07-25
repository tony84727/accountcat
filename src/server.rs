use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

use crate::{
    config,
    jwtutils::{self, Claims, JwtVerifier},
};

const SESSION_KEY_CLAIMS: &str = "claims";

struct ServerState {
    jwt_verify: JwtVerifier,
}

#[derive(Serialize, Deserialize)]
struct User {
    name: String,
}

async fn init_login(
    session: Session,
    State(state): State<Arc<ServerState>>,
    token: String,
) -> Result<Response, StatusCode> {
    let current_subject = session
        .get::<Claims>(SESSION_KEY_CLAIMS)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if current_subject.is_some() {
        return Ok(Redirect::to("/").into_response());
    }
    let claims = state
        .jwt_verify
        .verify(&token)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    session
        .insert(SESSION_KEY_CLAIMS, &claims)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(User { name: claims.name }).into_response())
}

async fn init_state() -> ServerState {
    let server_config = config::load().expect("load server config");
    let verifier = JwtVerifier::new(jwtutils::DEFAULT_JWK_URL, server_config.login.client_id)
        .await
        .expect("init jwt verifier");
    ServerState {
        jwt_verify: verifier,
    }
}

async fn get_name(session: Session) -> Result<String, StatusCode> {
    Ok(session
        .get::<Claims>(SESSION_KEY_CLAIMS)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|x| x.name)
        .unwrap_or_default())
}
pub async fn main() {
    tracing_subscriber::fmt::init();
    let serve_ui = ServeDir::new("ui/dist");
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store);
    let api_router = Router::new()
        .route("/login", post(init_login))
        .route("/name", get(get_name))
        .with_state(Arc::new(init_state().await))
        .layer(session_layer);

    let app = Router::new()
        .nest("/api", api_router)
        .fallback_service(serve_ui);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

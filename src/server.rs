use std::sync::Arc;

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

use crate::{
    config,
    jwtutils::{self, JwtVerifier},
};

struct ServerState {
    jwt_verify: JwtVerifier,
}

#[derive(Serialize, Deserialize)]
struct User {
    name: String,
}

async fn init_login(
    State(state): State<Arc<ServerState>>,
    token: String,
) -> Result<Json<User>, StatusCode> {
    let claims = state
        .jwt_verify
        .verify(&token)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(User { name: claims.name }))
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

pub async fn main() {
    tracing_subscriber::fmt::init();
    let serve_ui = ServeDir::new("ui/dist");
    let api_router = Router::new()
        .route("/login", post(init_login))
        .with_state(Arc::new(init_state().await));

    let app = Router::new()
        .nest("/api", api_router)
        .fallback_service(serve_ui);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

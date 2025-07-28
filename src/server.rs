use std::sync::Arc;

use axum::Router;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tonic_web::GrpcWebLayer;
use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use crate::{
    config::{self, Config},
    idl::user::user_server::UserServer,
    jwtutils::{self, JwtVerifier},
    user_service,
};

pub const SESSION_KEY_CLAIMS: &str = "claims";

pub struct ServerState {
    pub database: PgPool,
    pub jwt_verify: JwtVerifier,
}

async fn init_state() -> ServerState {
    let Config { login, database } = config::load().expect("load server config");
    let verifier = JwtVerifier::new(jwtutils::DEFAULT_JWK_URL, login.client_id)
        .await
        .expect("init jwt verifier");
    let connection = PgConnectOptions::from(database.unwrap_or_default());
    ServerState {
        jwt_verify: verifier,
        database: PgPoolOptions::new()
            .connect_with(connection)
            .await
            .expect("failed to connect database"),
    }
}

pub async fn main() {
    tracing_subscriber::fmt::init();
    let serve_ui = ServeDir::new("ui/dist");
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store);
    let server_state = Arc::new(init_state().await);
    let user_api = UserServer::new(user_service::UserApi::new(server_state.clone()));
    let mut grpc_server_builder = tonic::service::Routes::builder();
    grpc_server_builder.add_service(user_api);
    let grpc_server = grpc_server_builder.routes();

    let app = Router::new()
        .nest(
            "/api",
            grpc_server
                .into_axum_router()
                .layer(GrpcWebLayer::new())
                .layer(session_layer),
        )
        .fallback_service(serve_ui);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

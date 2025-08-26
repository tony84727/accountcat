use std::{path::PathBuf, sync::Arc};

use axum::Router;
use clap::Parser;
use http::header;
use sqlx::PgPool;
use tonic_web::GrpcWebLayer;
use tower::ServiceBuilder;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::PostgresStore;

use crate::{
    accounting_service,
    auth::FromSession,
    config::{self, Config},
    csp::{CspLayer, NonceLayer, build_csp},
    idl::{
        accounting::accounting_server::AccountingServer, todolist::todolist_server::TodolistServer,
        user::user_server::UserServer,
    },
    jwtutils::{self, JwtVerifier},
    serve_dist::ServeDist,
    todolist_service, user_service,
};

pub const SESSION_KEY_CLAIMS: &str = "claims";

pub struct ServerState {
    pub database: PgPool,
    pub jwt_verify: JwtVerifier,
}

pub async fn init_state(Config { login, database }: &Config) -> ServerState {
    let verifier = JwtVerifier::new(jwtutils::DEFAULT_JWK_URL, login.client_id.clone())
        .await
        .expect("init jwt verifier");
    ServerState {
        jwt_verify: verifier,
        database: database.clone().into(),
    }
}

#[derive(Parser, Default)]
pub struct ServerArg {
    /// Run database migration when starting the server
    #[arg(short, long)]
    auto_migrate: bool,
}

pub async fn main(arg: &ServerArg) {
    tracing_subscriber::fmt::init();
    let loaded_config = config::load().expect("load server config");
    let server_state = Arc::new(init_state(&loaded_config).await);
    if arg.auto_migrate {
        sqlx::migrate!("./migrations")
            .run(&server_state.database)
            .await
            .unwrap();
    }
    let serve_ui = ServeDist::new(PathBuf::from("ui/dist")).unwrap();
    let asset_service = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            build_csp(None),
        ))
        .layer(NonceLayer)
        .layer(CspLayer)
        .service(serve_ui);
    let session_store = PostgresStore::new(server_state.database.clone());
    let session_layer = SessionManagerLayer::new(session_store);
    let user_api = UserServer::new(user_service::UserApi::new(
        server_state.clone(),
        loaded_config.login.client_id,
    ));
    let id_claim_extractor = Arc::new(FromSession);
    let todolist_api = TodolistServer::new(todolist_service::TodolistApi::new(
        server_state.clone(),
        id_claim_extractor.clone(),
    ));
    let accounting_api = AccountingServer::new(accounting_service::AccountingApi::new(
        server_state.clone(),
        id_claim_extractor.clone(),
    ));
    let mut grpc_server_builder = tonic::service::Routes::builder();
    grpc_server_builder.add_service(user_api);
    grpc_server_builder.add_service(todolist_api);
    grpc_server_builder.add_service(accounting_api);
    let grpc_server = grpc_server_builder.routes();

    let app = Router::new()
        .nest(
            "/api",
            grpc_server
                .into_axum_router()
                .layer(GrpcWebLayer::new())
                .layer(session_layer),
        )
        .fallback_service(asset_service);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use axum::{Router, middleware as axum_middleware};
use clap::Parser;
use http::header;
use sqlx::PgPool;
use tokio::signal;
use tonic_web::GrpcWebLayer;
use tower::ServiceBuilder;
use tower_http::{
    set_header::SetResponseHeaderLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::PostgresStore;
use tracing::Level;

use crate::{
    config::Config,
    csp::{CspLayer, NonceLayer, build_csp},
    idl::{
        accounting::accounting_server::AccountingServer,
        instance_setting::instance_setting_server::InstanceSettingServer,
        todolist::todolist_server::TodolistServer, user::user_server::UserServer,
    },
    jwtutils::{self, JwtVerifier},
    middleware,
    serve_dist::ServeDist,
    service::{
        accounting::AccountingApi, instance_setting::InstanceSettingApi, todolist::TodolistApi,
        user::UserApi,
    },
};

pub const SESSION_KEY_CLAIMS: &str = "claims";

pub struct ServerState {
    pub database: PgPool,
    pub jwt_verify: JwtVerifier,
}

pub async fn init_state(
    Config {
        login, database, ..
    }: &Config,
) -> ServerState {
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

pub async fn main(arg: &ServerArg, config: &Config) {
    tracing_subscriber::fmt::init();
    let server_state = Arc::new(init_state(&config).await);
    if arg.auto_migrate {
        sqlx::migrate!("./migrations")
            .run(&server_state.database)
            .await
            .unwrap();
    }
    let serve_ui = ServeDist::new(PathBuf::from("ui/dist")).unwrap();
    let asset_service = ServiceBuilder::new()
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            build_csp(None),
        ))
        .layer(NonceLayer)
        .layer(CspLayer)
        .service(serve_ui);
    let session_store = PostgresStore::new(server_state.database.clone());
    let session_layer = SessionManagerLayer::new(session_store);
    let administrators = Arc::new(HashSet::from_iter(
        config.general.administrators.clone().unwrap_or_default(),
    ));
    let user_api = UserServer::new(UserApi::new(
        server_state.clone(),
        config.login.client_id.clone(),
        administrators.clone(),
    ));
    let todolist_api = TodolistServer::new(TodolistApi::new(server_state.clone()));
    let accounting_api = AccountingServer::new(AccountingApi::new(
        server_state.clone(),
        config.hashids.salt.clone(),
    ));
    let instance_setting_api = InstanceSettingServer::new(InstanceSettingApi::new(
        server_state.clone(),
        administrators.clone(),
    ));
    let mut grpc_server_builder = tonic::service::Routes::builder();
    grpc_server_builder.add_service(user_api);
    grpc_server_builder.add_service(todolist_api);
    grpc_server_builder.add_service(accounting_api);
    grpc_server_builder.add_service(instance_setting_api);
    let grpc_server = grpc_server_builder.routes();

    let identity_layer = axum_middleware::from_fn(middleware::identity::enforce_identity);
    let app = Router::new()
        .nest(
            "/api",
            grpc_server
                .clone()
                .into_axum_router()
                .layer(
                    TraceLayer::new_for_grpc()
                        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                        .on_response(DefaultOnResponse::new().level(Level::INFO)),
                )
                .layer(identity_layer.clone())
                .layer(GrpcWebLayer::new())
                .layer(session_layer.clone()),
        )
        .nest(
            "/grpc",
            grpc_server
                .into_axum_router()
                .layer(
                    TraceLayer::new_for_grpc()
                        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                        .on_response(DefaultOnResponse::new().level(Level::INFO)),
                )
                .layer(identity_layer)
                .layer(session_layer),
        )
        .fallback_service(asset_service);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.server.port.unwrap_or(3000)));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

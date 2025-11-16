use std::sync::Arc;

use accountcat::{
    config::{Config, General, HashIds, Login, Pki},
    idl::user::{user_client::UserClient, user_server::UserServer},
    middleware::identity,
    server::{ServerState, init_state},
    service::user::UserApi,
    testing::{self, test_database::TestDatabase},
};
use secrecy::SecretString;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn init_test_database_and_server_state() -> (TestDatabase, ServerState) {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let server_state = init_state(&Config {
        general: General::default(),
        login: Login {
            client_id: SecretString::from("dummy"),
        },
        database: database.clone(),
        hashids: HashIds {
            salt: SecretString::from("dummy"),
        },
        pki: Pki::default(),
    })
    .await;
    (test_database, server_state)
}

#[tokio::test]
async fn test_get_param_doesnt_require_authentication() {
    let (_test_database, server_state) = init_test_database_and_server_state().await;
    let user_api = UserServer::new(UserApi::new(
        Arc::new(server_state),
        SecretString::from("dummy"),
        Default::default(),
    ));
    let identity_layer = axum::middleware::from_fn(identity::enforce_identity);
    let mut grpc_server_builder = tonic::service::Routes::builder();
    grpc_server_builder.add_service(user_api);
    let router = axum::Router::new().fallback_service(
        grpc_server_builder
            .routes()
            .into_axum_router()
            .layer(identity_layer)
            .layer(SessionManagerLayer::new(MemoryStore::default())),
    );
    let mut client = UserClient::new(router);
    let response = client.get_param(()).await.unwrap().into_inner();
    assert_eq!("dummy", response.google_client_id);
    assert_eq!(None, response.announcement);
}

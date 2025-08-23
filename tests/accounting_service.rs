use std::sync::Arc;

use accountcat::{
    accounting_service::AccountingApi,
    config::{self, Config, Login},
    idl::accounting::{Amount, NewItem, accounting_server::Accounting},
    server::init_state,
    testing::{
        DummyIdClaimExtractor, insert_fake_user, test_database_manager::TestDatabaseManager,
    },
};
use secrecy::SecretString;
use tonic::Request;

#[tokio::test]
async fn test_add_accounting_item() {
    tracing_subscriber::fmt::init();
    let config = config::load().unwrap();
    let mut test_manager = TestDatabaseManager::new(
        String::from("accountcat-testing-"),
        config.database.clone(),
        config.database.into(),
    );
    let database = test_manager.create().await.unwrap();
    let server_state = init_state(&Config {
        login: Login {
            client_id: SecretString::from("dummy"),
        },
        database,
    })
    .await;
    insert_fake_user(&server_state.database).await.unwrap();
    let accounting_api = AccountingApi::new(
        Arc::new(server_state),
        Arc::new(DummyIdClaimExtractor::new(String::from("testing"))),
    );

    let test_add = async |amount: &'static str| {
        let req = Request::new(NewItem {
            name: String::from("test item"),
            amount: Some(Amount {
                amount: String::from(amount),
                currency: String::from("TWD"),
            }),
            tags: Default::default(),
        });
        let response = accounting_api.add(req).await.unwrap();
        assert_eq!(
            amount,
            response.into_inner().amount.map(|x| x.amount).unwrap(),
            "insert {}",
            amount,
        );
    };
    test_add("100.1").await;
    test_add("100000.1").await;
    test_add("999999.99").await;
}

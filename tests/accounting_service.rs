use std::sync::Arc;

use accountcat::{
    accounting_service::AccountingApi,
    config::{self, Config, Login},
    idl::accounting::{Amount, AmountType, Item, NewItem, accounting_server::Accounting},
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

    let test_add =
        async |amount: &'static str, amount_type: AmountType, expected_amount: &'static str| {
            let req = Request::new(NewItem {
                name: String::from("test item"),
                amount: Some(Amount {
                    amount: String::from(amount),
                    currency: String::from("TWD"),
                }),
                r#type: amount_type as i32,
                tags: Default::default(),
            });
            let response = accounting_api.add(req).await.unwrap();
            assert_eq!(
                expected_amount,
                response.into_inner().amount.map(|x| x.amount).unwrap(),
                "insert {amount_type:?} {} should return with amount {expected_amount}",
                amount,
            );
        };
    // positive income represents an income
    test_add("999999.99", AmountType::Income, "999999.99").await;
    // negative income represents an expense
    test_add("-999999.99", AmountType::Income, "-999999.99").await;
    // negative expense represents an income
    test_add("-999999.99", AmountType::Expense, "999999.99").await;
    // positive expense represents an expense
    test_add("999999.99", AmountType::Expense, "-999999.99").await;
    let list = accounting_api
        .list(Request::new(()))
        .await
        .unwrap()
        .into_inner();
    let expected = vec![
        (String::from("-999999.99"), AmountType::Expense as i32),
        (String::from("999999.99"), AmountType::Income as i32),
        (String::from("-999999.99"), AmountType::Expense as i32),
        (String::from("999999.99"), AmountType::Income as i32),
    ];
    assert_eq!(
        expected,
        list.items
            .into_iter()
            .map(|Item { amount, r#type, .. }| { (amount.unwrap().amount, r#type) })
            .collect::<Vec<(String, i32)>>()
    )
}

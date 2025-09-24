use std::sync::Arc;

use accountcat::{
    config::{Config, General, HashIds, Login},
    idl::accounting::{
        Amount, AmountType, Item, ItemList, NewItem, UpdateItemRequest,
        accounting_server::Accounting,
    },
    protobufutils::to_proto_timestamp,
    server::{ServerState, init_state},
    service::accounting::AccountingApi,
    testing::{self, DummyIdClaimExtractor, insert_fake_user, test_database::TestDatabase},
};
use secrecy::SecretString;
use time::OffsetDateTime;
use tonic::Request;

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
    })
    .await;
    (test_database, server_state)
}

#[tokio::test]
async fn test_add_accounting_item() {
    let (_test_database, server_state) = init_test_database_and_server_state().await;
    insert_fake_user(&server_state.database).await.unwrap();
    let accounting_api = AccountingApi::new(
        Arc::new(server_state),
        Arc::new(DummyIdClaimExtractor::new(String::from("testing"))),
        SecretString::from("dummy"),
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

#[tokio::test]
async fn test_update_accounting_item_occurred_at() {
    let (_test_database, server_state) = init_test_database_and_server_state().await;
    insert_fake_user(&server_state.database).await.unwrap();
    let accounting_api = AccountingApi::new(
        Arc::new(server_state),
        Arc::new(DummyIdClaimExtractor::new(String::from("testing"))),
        SecretString::from("dummy"),
    );

    let req = Request::new(NewItem {
        name: String::from("test item"),
        amount: Some(Amount {
            amount: String::from("100"),
            currency: String::from("TWD"),
        }),
        r#type: AmountType::Expense as i32,
        tags: Default::default(),
    });
    let _response = accounting_api.add(req).await.unwrap();
    let list_items = || async {
        let list = accounting_api.list(Request::new(())).await.unwrap();
        let ItemList { items } = list.into_inner();
        items
    };
    let items = list_items().await;
    assert_eq!(1, items.len());
    let original_item = items.first().unwrap();
    let item_id = original_item.id.clone();
    let _response = accounting_api
        .update_item(Request::new(UpdateItemRequest {
            id: item_id,
            name: None,
            amount: None,
            occurred_at: Some(to_proto_timestamp(
                OffsetDateTime::from_unix_timestamp(1753599600).unwrap(),
            )),
        }))
        .await
        .unwrap();
    let items = list_items().await;
    assert_eq!(1, items.len());
    let item = items.first().unwrap();
    assert_eq!(original_item.name, item.name);
    assert_eq!(original_item.created_at, item.created_at);
    assert_eq!(original_item.amount, item.amount);
    assert_eq!(1753599600, item.occurred_at.map(|t| t.seconds).unwrap());
}

async fn test_update_accounting_item_amount_magnitude(
    amount_type: AmountType,
    origin: &str,
    modified: &str,
    expected: &str,
) {
    let (_test_database, server_state) = init_test_database_and_server_state().await;
    insert_fake_user(&server_state.database).await.unwrap();
    let accounting_api = AccountingApi::new(
        Arc::new(server_state),
        Arc::new(DummyIdClaimExtractor::new(String::from("testing"))),
        SecretString::from("dummy"),
    );

    let req = Request::new(NewItem {
        name: String::from("test item"),
        amount: Some(Amount {
            amount: String::from(origin),
            currency: String::from("TWD"),
        }),
        r#type: amount_type as i32,
        tags: Default::default(),
    });
    let _response = accounting_api.add(req).await.unwrap();
    let list_items = || async {
        let list = accounting_api.list(Request::new(())).await.unwrap();
        let ItemList { items } = list.into_inner();
        items
    };
    let items = list_items().await;
    assert_eq!(1, items.len());
    let original_item = items.first().unwrap();
    let item_id = original_item.id.clone();
    let _response = accounting_api
        .update_item(Request::new(UpdateItemRequest {
            id: item_id,
            name: None,
            amount: Some(Amount {
                currency: String::from("TWD"),
                amount: String::from(modified),
            }),
            occurred_at: None,
        }))
        .await
        .unwrap();
    let items = list_items().await;
    assert_eq!(1, items.len());
    let item = items.first().unwrap();
    assert_eq!(original_item.name, item.name);
    assert_eq!(original_item.occurred_at, item.occurred_at);
    assert_eq!(original_item.created_at, item.created_at);
    assert_eq!(
        expected,
        item.amount
            .as_ref()
            .map(|amount| amount.amount.clone())
            .unwrap()
    );
}

#[tokio::test]
async fn test_update_accounting_item_amount_magnitude_expense() {
    test_update_accounting_item_amount_magnitude(AmountType::Expense, "100", "1000", "-1000").await;
}

#[tokio::test]
async fn test_update_accounting_item_amount_magnitude_income() {
    test_update_accounting_item_amount_magnitude(AmountType::Income, "100", "1000", "1000").await;
}

#[tokio::test]
async fn test_update_accounting_item_amount_magnitude_zero_income() {
    test_update_accounting_item_amount_magnitude(AmountType::Income, "0", "1000", "1000").await;
}

#[tokio::test]
async fn test_update_accounting_item_name() {
    let (_test_database, server_state) = init_test_database_and_server_state().await;
    insert_fake_user(&server_state.database).await.unwrap();
    let accounting_api = AccountingApi::new(
        Arc::new(server_state),
        Arc::new(DummyIdClaimExtractor::new(String::from("testing"))),
        SecretString::from("dummy"),
    );

    let req = Request::new(NewItem {
        name: String::from("test item"),
        amount: Some(Amount {
            amount: String::from("100"),
            currency: String::from("TWD"),
        }),
        r#type: AmountType::Expense as i32,
        tags: Default::default(),
    });
    let _response = accounting_api.add(req).await.unwrap();
    let list_items = || async {
        let list = accounting_api.list(Request::new(())).await.unwrap();
        let ItemList { items } = list.into_inner();
        items
    };
    let items = list_items().await;
    assert_eq!(1, items.len());
    let original_item = items.first().unwrap();
    let item_id = original_item.id.clone();
    let _response = accounting_api
        .update_item(Request::new(UpdateItemRequest {
            id: item_id,
            name: Some(String::from("test item1")),
            amount: None,
            occurred_at: None,
        }))
        .await
        .unwrap();
    let items = list_items().await;
    assert_eq!(1, items.len());
    let item = items.first().unwrap();
    assert_eq!("test item1", item.name);
    assert_eq!(original_item.occurred_at, item.occurred_at);
    assert_eq!(original_item.created_at, item.created_at);
    assert_eq!(original_item.amount, item.amount);
}

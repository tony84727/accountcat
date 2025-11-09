use std::{collections::HashSet, sync::Arc};

use accountcat::{
    config::{Config, General, HashIds, Login, Pki},
    idl::instance_setting::{Announcement, instance_setting_server::InstanceSetting},
    server::{ServerState, init_state},
    service::instance_setting::InstanceSettingApi,
    testing::{self, DummyIdClaimExtractor, test_database::TestDatabase},
};
use secrecy::SecretString;
use tonic::Request;

async fn init_test_database_and_server_state() -> (TestDatabase, ServerState) {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let server_state = init_state(&Config {
        general: General {
            administrators: Some(vec![String::from("testing")]),
        },
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
async fn test_add_announcement() {
    let (_test_database, server_state) = init_test_database_and_server_state().await;
    let server_state = Arc::new(server_state);
    let instacne_setting_api = InstanceSettingApi::new(
        server_state.clone(),
        Arc::new(DummyIdClaimExtractor::new(String::from("testing"))),
        Arc::from({
            let mut set = HashSet::new();
            set.insert(String::from("testing"));
            set
        }),
    );
    let req = Request::new(Announcement {
        content: String::from("announcement of website"),
    });
    instacne_setting_api.set_announcement(req).await.unwrap();
    let latest_announcement = sqlx::query!(
        "select content from announcements where hidden_at is null order by created_at desc limit 1"
    )
    .fetch_one(&server_state.database)
    .await
    .unwrap();
    assert_eq!("announcement of website", latest_announcement.content);
    let row = sqlx::query!("select count(*) from announcements")
        .fetch_one(&server_state.database)
        .await
        .unwrap();
    assert_eq!(Some(1), row.count);
    let req = Request::new(Announcement {
        content: String::from("new announcement of website"),
    });
    instacne_setting_api.set_announcement(req).await.unwrap();
    let latest_announcement = sqlx::query!(
        "select content from announcements where hidden_at is null order by created_at desc, id desc limit 1"
    )
    .fetch_one(&server_state.database)
    .await
    .unwrap();
    assert_eq!("new announcement of website", latest_announcement.content);
    let row = sqlx::query!("select count(*) from announcements")
        .fetch_one(&server_state.database)
        .await
        .unwrap();
    assert_eq!(Some(2), row.count);
    let row = sqlx::query!("select count(*) from announcements where hidden_at is null")
        .fetch_one(&server_state.database)
        .await
        .unwrap();
    assert_eq!(Some(1), row.count);
}

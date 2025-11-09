use accountcat::{
    pki::ca::{CertificateAuthority, CertificateIssuer, TrackedCertificateIssuer},
    testing::{self, test_database::TestDatabase},
};
use sqlx::PgPool;
use time::Duration;

#[tokio::test]
async fn test_store_issued_certificate() {
    let test_database = testing::create_database().await;
    let ca = CertificateAuthority::generate().unwrap();
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    let tracked = TrackedCertificateIssuer::new(database_connection.clone(), ca);
    tracked
        .issue("testing client", Duration::seconds(60))
        .await
        .unwrap();
    let row = sqlx::query!("select count(*) from certificates")
        .fetch_one(&database_connection)
        .await
        .unwrap();
    assert_eq!(Some(1), row.count);
}

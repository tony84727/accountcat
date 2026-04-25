use accountcat::{
    pki::ca::{CertificateAuthority, CertificateIssuer, LoadError, TrackedCertificateIssuer},
    testing::{self, test_database::TestDatabase},
};
use sqlx::{PgPool, Row};
use time::Duration;

#[tokio::test]
async fn test_initialize_stores_ca_certificate() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    CertificateAuthority::initialize(&database_connection)
        .await
        .unwrap();
    let row = sqlx::query(
        "select
            id,
            certificates.is_ca,
            certificates.trusted,
            certificates.private_key_der,
            certificates.issuer_certificate_id
        from certificates",
    )
    .fetch_one(&database_connection)
    .await
    .unwrap();
    assert!(row.try_get::<bool, _>("is_ca").unwrap());
    assert!(row.try_get::<bool, _>("trusted").unwrap());
    assert!(
        row.try_get::<Option<Vec<u8>>, _>("private_key_der")
            .unwrap()
            .is_some()
    );
    assert!(
        row.try_get::<Option<i32>, _>("issuer_certificate_id")
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_initialize_allows_multiple_trusted_cas() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    CertificateAuthority::initialize(&database_connection)
        .await
        .unwrap();
    CertificateAuthority::initialize(&database_connection)
        .await
        .unwrap();
    let row = sqlx::query("select count(*) as count from certificates where is_ca and trusted")
        .fetch_one(&database_connection)
        .await
        .unwrap();
    assert_eq!(2_i64, row.try_get::<i64, _>("count").unwrap());
}

#[tokio::test]
async fn test_load_by_id_loads_requested_ca() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    CertificateAuthority::initialize(&database_connection)
        .await
        .unwrap();
    let expected_id = sqlx::query_scalar::<_, i32>(
        "select id from certificates where is_ca and trusted order by created_at desc, id desc limit 1",
    )
    .fetch_one(&database_connection)
    .await
    .unwrap();

    let ca = CertificateAuthority::load_by_id(&database_connection, expected_id)
        .await
        .unwrap();

    assert_eq!(ca.issuer_certificate_id(), Some(expected_id));
}

#[tokio::test]
async fn test_load_by_id_rejects_missing_issuer() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();

    let err = CertificateAuthority::load_by_id(&database_connection, 999_999)
        .await
        .unwrap_err();

    assert!(matches!(err, LoadError::MissingIssuer(999_999)));
}

#[tokio::test]
async fn test_load_by_id_rejects_non_issuable_certificate() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    let certificate_id = sqlx::query_scalar::<_, i32>(
        "insert into certificates (
            serial,
            not_before,
            not_after,
            is_ca,
            trusted
        ) values ($1, now(), now() + interval '1 day', false, false)
        returning id",
    )
    .bind(1_i32)
    .fetch_one(&database_connection)
    .await
    .unwrap();

    let err = CertificateAuthority::load_by_id(&database_connection, certificate_id)
        .await
        .unwrap_err();

    assert!(matches!(err, LoadError::IssuerNotUsable(id) if id == certificate_id));
}

#[tokio::test]
async fn test_store_issued_certificate() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    let ca = CertificateAuthority::initialize(&database_connection)
        .await
        .unwrap();
    let tracked = TrackedCertificateIssuer::new(database_connection.clone(), ca);
    tracked
        .issue("testing client", Duration::seconds(60))
        .await
        .unwrap();
    let row = sqlx::query(
        "select
            count(*) as count,
            count(*) filter (where der is not null and issuer_certificate_id is not null and private_key_der is not null and not is_ca) as issued_count,
            count(*) filter (where issuer_certificate_id is null and is_ca and trusted) as root_count
        from certificates",
    )
    .fetch_one(&database_connection)
    .await
    .unwrap();
    assert_eq!(2_i64, row.try_get::<i64, _>("count").unwrap());
    assert_eq!(1_i64, row.try_get::<i64, _>("issued_count").unwrap());
    assert_eq!(1_i64, row.try_get::<i64, _>("root_count").unwrap());
}

use accountcat::{
    pki::{
        ca::{
            CertificateAuthority, CertificateIssuer, ImportCertificateError, LoadError,
            TrackedCertificateIssuer, create_pending_csr, import_certificate,
        },
        csr::ToBeSignedCertificate,
    },
    testing::{self, test_database::TestDatabase},
};
use rcgen::{CertificateParams, KeyPair, PKCS_ED25519};
use sqlx::{PgPool, Row};
use time::{Duration, OffsetDateTime};
use x509_parser::{certification_request::X509CertificationRequest, prelude::FromDer};

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

#[tokio::test]
async fn test_create_pending_csr_stores_trusted_pending_row() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();

    let pending = create_pending_csr(&database_connection, "pending client", Duration::days(90))
        .await
        .unwrap();

    let (_, parsed) = X509CertificationRequest::from_der(&pending.request_der).unwrap();
    assert_eq!(
        parsed.certification_request_info.subject_pki.raw,
        ToBeSignedCertificate::create_with_key(
            "pending client",
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc().saturating_add(Duration::days(1)),
            KeyPair::try_from(pending.private_key_der.clone()).unwrap(),
        )
        .subject_public_key_info()
    );
    assert!(pending.request_pem.contains("BEGIN CERTIFICATE REQUEST"));

    let row = sqlx::query(
        "select
            der,
            private_key_der,
            serial,
            not_before,
            not_after,
            trusted,
            is_ca,
            subject_key_id
        from certificates
        where id = $1",
    )
    .bind(pending.id)
    .fetch_one(&database_connection)
    .await
    .unwrap();
    assert!(row.try_get::<Option<Vec<u8>>, _>("der").unwrap().is_none());
    assert!(
        row.try_get::<Option<Vec<u8>>, _>("private_key_der")
            .unwrap()
            .is_some()
    );
    assert!(
        row.try_get::<Option<sqlx::types::BigDecimal>, _>("serial")
            .unwrap()
            .is_none()
    );
    assert!(
        row.try_get::<Option<OffsetDateTime>, _>("not_before")
            .unwrap()
            .is_none()
    );
    assert!(
        row.try_get::<Option<OffsetDateTime>, _>("not_after")
            .unwrap()
            .is_none()
    );
    assert!(row.try_get::<bool, _>("trusted").unwrap());
    assert!(!row.try_get::<bool, _>("is_ca").unwrap());
    assert_eq!(
        Some(pending.subject_key_id),
        row.try_get::<Option<Vec<u8>>, _>("subject_key_id").unwrap()
    );
}

#[tokio::test]
async fn test_import_certificate_updates_pending_csr_row() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    let pending = create_pending_csr(&database_connection, "pending client", Duration::days(90))
        .await
        .unwrap();
    let ca = CertificateAuthority::initialize(&database_connection)
        .await
        .unwrap();
    let key = KeyPair::try_from(pending.private_key_der.clone()).unwrap();
    let tbs = ToBeSignedCertificate::create_with_key(
        "pending client",
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc().saturating_add(Duration::days(90)),
        key,
    );
    let issued = ca.sign(tbs).unwrap();

    let imported = import_certificate(&database_connection, issued.certificate.der())
        .await
        .unwrap();

    assert_eq!(pending.id, imported.id);
    assert!(!imported.created);
    assert!(!imported.idempotent);
    let row = sqlx::query(
        "select der, serial, not_before, not_after, trusted, issuer_certificate_id
        from certificates
        where id = $1",
    )
    .bind(pending.id)
    .fetch_one(&database_connection)
    .await
    .unwrap();
    assert!(row.try_get::<Option<Vec<u8>>, _>("der").unwrap().is_some());
    assert!(
        row.try_get::<Option<sqlx::types::BigDecimal>, _>("serial")
            .unwrap()
            .is_some()
    );
    assert!(
        row.try_get::<Option<OffsetDateTime>, _>("not_before")
            .unwrap()
            .is_some()
    );
    assert!(
        row.try_get::<Option<OffsetDateTime>, _>("not_after")
            .unwrap()
            .is_some()
    );
    assert!(row.try_get::<bool, _>("trusted").unwrap());
    assert!(
        row.try_get::<Option<i32>, _>("issuer_certificate_id")
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn test_import_certificate_rejects_private_key_mismatch() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    let pending = create_pending_csr(&database_connection, "pending client", Duration::days(90))
        .await
        .unwrap();
    let ca = CertificateAuthority::initialize(&database_connection)
        .await
        .unwrap();
    let tbs = ToBeSignedCertificate::create(
        "pending client",
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc().saturating_add(Duration::days(90)),
    )
    .unwrap();
    let subject_key_id = tbs.subject_key_id();
    let issued = ca.sign(tbs).unwrap();
    sqlx::query("update certificates set subject_key_id = $1 where id = $2")
        .bind(subject_key_id)
        .bind(pending.id)
        .execute(&database_connection)
        .await
        .unwrap();

    let err = import_certificate(&database_connection, issued.certificate.der())
        .await
        .unwrap_err();

    assert!(matches!(err, ImportCertificateError::PublicKeyMismatch));
}

#[tokio::test]
async fn test_import_certificate_is_idempotent_for_identical_der() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    let pending = create_pending_csr(&database_connection, "pending client", Duration::days(90))
        .await
        .unwrap();
    let ca = CertificateAuthority::initialize(&database_connection)
        .await
        .unwrap();
    let key = KeyPair::try_from(pending.private_key_der).unwrap();
    let issued = ca
        .sign(ToBeSignedCertificate::create_with_key(
            "pending client",
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc().saturating_add(Duration::days(90)),
            key,
        ))
        .unwrap();
    import_certificate(&database_connection, issued.certificate.der())
        .await
        .unwrap();

    let imported = import_certificate(&database_connection, issued.certificate.der())
        .await
        .unwrap();

    assert_eq!(pending.id, imported.id);
    assert!(imported.idempotent);
}

#[tokio::test]
async fn test_import_certificate_rejects_different_der_for_existing_subject_key_id() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    let pending = create_pending_csr(&database_connection, "pending client", Duration::days(90))
        .await
        .unwrap();
    let ca = CertificateAuthority::initialize(&database_connection)
        .await
        .unwrap();
    let first_key = KeyPair::try_from(pending.private_key_der.clone()).unwrap();
    let first = ca
        .sign(ToBeSignedCertificate::create_with_key(
            "pending client",
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc().saturating_add(Duration::days(90)),
            first_key,
        ))
        .unwrap();
    import_certificate(&database_connection, first.certificate.der())
        .await
        .unwrap();
    let second_key = KeyPair::try_from(pending.private_key_der).unwrap();
    let second = ca
        .sign(ToBeSignedCertificate::create_with_key(
            "pending client",
            OffsetDateTime::now_utc().saturating_add(Duration::days(1)),
            OffsetDateTime::now_utc().saturating_add(Duration::days(91)),
            second_key,
        ))
        .unwrap();

    let err = import_certificate(&database_connection, second.certificate.der())
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ImportCertificateError::CertificateAlreadyImported(id) if id == pending.id
    ));
}

#[tokio::test]
async fn test_import_certificate_without_matching_row_creates_untrusted_row() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    let ca = CertificateAuthority::initialize(&database_connection)
        .await
        .unwrap();
    let issued = ca
        .issue("external client", Duration::days(90))
        .await
        .unwrap();

    let imported = import_certificate(&database_connection, issued.certificate.der())
        .await
        .unwrap();

    assert!(imported.created);
    let trusted = sqlx::query_scalar::<_, bool>("select trusted from certificates where id = $1")
        .bind(imported.id)
        .fetch_one(&database_connection)
        .await
        .unwrap();
    assert!(!trusted);
}

#[tokio::test]
async fn test_import_certificate_rejects_certificate_without_subject_key_identifier() {
    let test_database = testing::create_database().await;
    let TestDatabase { database } = &test_database;
    let database_connection: PgPool = database.clone().into();
    let key = KeyPair::generate_for(&PKCS_ED25519).unwrap();
    let params = CertificateParams::new(vec![String::from("no-ski.example")]).unwrap();
    let certificate = params.self_signed(&key).unwrap();

    let err = import_certificate(&database_connection, certificate.der())
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ImportCertificateError::MissingSubjectKeyIdentifier
    ));
}

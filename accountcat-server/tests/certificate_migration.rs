use accountcat::{config::Config, testing::test_database::TestDatabase};
use sqlx::{Executor, PgPool, Row, types::BigDecimal};

#[tokio::test]
async fn test_certificate_schema_migration_preserves_legacy_rows() {
    let config = Config::load(None).unwrap();
    let db = TestDatabase::new_without_migrations(
        String::from("accountcat-migration-testing-"),
        config.database,
    )
    .await
    .unwrap();
    let pool: PgPool = db.pool();
    pool.execute(include_str!(
        "../migrations/20251108091248_add_certificates_table.up.sql"
    ))
    .await
    .unwrap();
    pool.execute(
        "insert into certificates (
                serial,
                dn,
                country,
                state,
                locality,
                organization,
                organizational_unit,
                common_name,
                not_before,
                not_after
            ) values (
                1001,
                '\\x0102',
                'TW',
                null,
                null,
                'Accountcat project',
                null,
                'legacy cert',
                now(),
                now() + interval '1 day'
            )",
    )
    .await
    .unwrap();
    pool.execute(include_str!(
        "../migrations/20251118000000_track_certificate_authorities.up.sql"
    ))
    .await
    .unwrap();

    let row = sqlx::query(
        "select
            serial,
            der,
            private_key_der,
            issuer_certificate_id,
            is_ca,
            trusted
        from certificates
        where common_name = 'legacy cert'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        "1001",
        row.try_get::<BigDecimal, _>("serial").unwrap().to_string()
    );
    assert!(row.try_get::<Option<Vec<u8>>, _>("der").unwrap().is_none());
    assert!(
        row.try_get::<Option<Vec<u8>>, _>("private_key_der")
            .unwrap()
            .is_none()
    );
    assert!(
        row.try_get::<Option<i32>, _>("issuer_certificate_id")
            .unwrap()
            .is_none()
    );
    assert!(!row.try_get::<bool, _>("is_ca").unwrap());
    assert!(!row.try_get::<bool, _>("trusted").unwrap());
}

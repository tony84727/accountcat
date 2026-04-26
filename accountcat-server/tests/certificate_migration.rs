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
    pool.execute(include_str!(
        "../migrations/20251126000000_track_subject_key_ids.up.sql"
    ))
    .await
    .unwrap();

    let row = sqlx::query!(
        "select
            serial,
            der,
            private_key_der,
            issuer_certificate_id,
            is_ca,
            trusted,
            subject_key_id
        from certificates
        where common_name = 'legacy cert'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!("1001", row.serial.unwrap().to_string());
    assert!(row.der.is_none());
    assert!(row.private_key_der.is_none());
    assert!(row.issuer_certificate_id.is_none());
    assert!(!row.is_ca);
    assert!(!row.trusted);
    assert!(row.subject_key_id.is_none());
}

#[tokio::test]
async fn test_certificate_schema_migration_allows_pending_csr_rows() {
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
    pool.execute(include_str!(
        "../migrations/20251118000000_track_certificate_authorities.up.sql"
    ))
    .await
    .unwrap();
    pool.execute(include_str!(
        "../migrations/20251126000000_track_subject_key_ids.up.sql"
    ))
    .await
    .unwrap();

    let id = sqlx::query_scalar::<_, i32>(
        "insert into certificates (
            serial,
            common_name,
            not_before,
            not_after,
            private_key_der,
            trusted,
            subject_key_id
        ) values (
            null,
            'pending csr',
            null,
            null,
            '\\x01',
            true,
            '\\x0203'
        )
        returning id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let row = sqlx::query(
        "select
            serial,
            not_before,
            not_after,
            subject_key_id,
            count(*) over () as matching_indexes
        from certificates
        where id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        row.try_get::<Option<BigDecimal>, _>("serial")
            .unwrap()
            .is_none()
    );
    assert!(
        row.try_get::<Option<time::OffsetDateTime>, _>("not_before")
            .unwrap()
            .is_none()
    );
    assert!(
        row.try_get::<Option<time::OffsetDateTime>, _>("not_after")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        Some(vec![2_u8, 3_u8]),
        row.try_get::<Option<Vec<u8>>, _>("subject_key_id").unwrap()
    );

    let has_index = sqlx::query_scalar::<_, bool>(
        "select exists (
            select 1
            from pg_indexes
            where tablename = 'certificates'
                and indexname = 'certificates_subject_key_id'
        )",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(has_index);
}

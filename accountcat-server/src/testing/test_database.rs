use sqlx::{Executor, PgPool};
use uuid::Uuid;

use crate::config::Database;

pub struct TestDatabase {
    pub database: Database,
}

impl TestDatabase {
    pub async fn new_without_migrations(
        prefix: String,
        database_config: Database,
    ) -> sqlx::Result<Self> {
        let id = Uuid::new_v4();
        let database_name = format!("{}{}", prefix, id);
        let pool: PgPool = database_config.clone().into();
        pool.execute(format!(r#"create database "{database_name}""#).as_str())
            .await?;
        let mut config = database_config;
        config.database = Some(database_name);
        Ok(Self { database: config })
    }

    pub async fn new(prefix: String, database_config: Database) -> sqlx::Result<Self> {
        let test_database = Self::new_without_migrations(prefix, database_config).await?;
        let pool = test_database.database.clone().into();
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(test_database)
    }

    pub fn pool(&self) -> PgPool {
        self.database.clone().into()
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        let Some(database) = self.database.database.clone() else {
            return;
        };
        let admin_database = self.database.clone().without_name();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let pool = PgPool::from(admin_database);
                pool.execute(format!(r#"drop database "{database}" with (FORCE)"#).as_str())
                    .await
                    .unwrap();
            });
        })
        .join()
        .unwrap();
    }
}

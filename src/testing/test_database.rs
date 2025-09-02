use sqlx::{Executor, PgPool};
use uuid::Uuid;

use crate::config::Database;

pub struct TestDatabase {
    pub database: Database,
}

impl TestDatabase {
    pub async fn new(prefix: String, database_config: Database) -> sqlx::Result<Self> {
        let id = Uuid::new_v4();
        let database_name = format!("{}{}", prefix, id);
        let pool: PgPool = database_config.clone().into();
        pool.execute(format!(r#"create database "{database_name}""#).as_str())
            .await?;
        let mut config = database_config;
        config.database = Some(database_name);
        let pool = config.clone().into();
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { database: config })
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        let Some(database) = self.database.database.clone() else {
            return;
        };
        let pool = PgPool::from(self.database.clone().without_name());
        std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                pool.execute(format!(r#"drop database "{database}" with (FORCE)"#).as_str())
                    .await
                    .unwrap();
            });
        })
        .join()
        .unwrap();
    }
}

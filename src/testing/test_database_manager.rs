use sqlx::{Executor, PgPool};
use uuid::Uuid;

use crate::config::Database;

pub struct TestDatabaseManager {
    prefix: String,
    pool: PgPool,
    database_config: Database,
    databases: Vec<String>,
}

impl TestDatabaseManager {
    pub fn new(prefix: String, database_config: Database, pool: PgPool) -> TestDatabaseManager {
        Self {
            prefix,
            pool,
            database_config,
            databases: Vec::new(),
        }
    }

    pub async fn create(&mut self) -> Result<Database, sqlx::Error> {
        let id = Uuid::new_v4();
        let database_name = format!("{}{}", self.prefix, id);
        self.pool
            .execute(format!(r#"create database "{database_name}""#).as_str())
            .await?;
        self.databases.push(database_name.clone());
        let mut config = self.database_config.clone();
        config.database = Some(database_name);
        let pool = config.clone().into();
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(config)
    }

    pub async fn clean(&mut self) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for database in self.databases.iter() {
            tx.execute(format!(r#"drop database "{database}""#).as_str())
                .await?;
        }
        tx.commit().await?;
        self.databases = Vec::new();
        Ok(())
    }
}

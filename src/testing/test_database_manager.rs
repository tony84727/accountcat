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
        let databases = std::mem::take(&mut self.databases);
        for database in databases.iter() {
            drop_database(&self.pool, database).await?;
        }
        Ok(())
    }
}

async fn drop_database(pool: &PgPool, database: &str) -> sqlx::Result<()> {
    pool.execute(format!(r#"drop database "{database}" with (FORCE)"#).as_str())
        .await?;
    Ok(())
}

impl Drop for TestDatabaseManager {
    fn drop(&mut self) {
        let Self {
            databases,
            database_config,
            ..
        } = self;
        if databases.is_empty() {
            return;
        }
        let pool: PgPool = database_config.clone().into();
        let clean = futures::future::join_all(databases.iter().map(|database| {
            let pool = pool.clone();
            let database = database.clone();
            async move { drop_database(&pool, &database).await }
        }));
        std::thread::spawn(move || {
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                rt.block_on(clean);
            }
        })
        .join()
        .unwrap();
    }
}

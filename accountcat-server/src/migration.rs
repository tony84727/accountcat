use sqlx::PgPool;

use crate::config::Config;

pub async fn run(config: &Config) {
    let pool: PgPool = config.database.clone().into();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
}

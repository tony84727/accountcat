use sqlx::PgPool;

use crate::config;

pub async fn run() {
    let config = config::load().unwrap();
    let pool: PgPool = config.database.into();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
}

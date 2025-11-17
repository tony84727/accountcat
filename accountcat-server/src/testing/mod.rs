use sqlx::PgPool;
use tonic::Request;

use crate::{config::Config, jwtutils::Claims};

pub mod cwd;
pub mod test_database;
use test_database::TestDatabase;

pub fn test_claims(sub: String) -> Claims {
    Claims {
        iss: Default::default(),
        azp: Default::default(),
        aud: Default::default(),
        sub,
        iat: Default::default(),
        exp: Default::default(),
        picture: Default::default(),
        given_name: Default::default(),
        family_name: Default::default(),
        name: Default::default(),
    }
}

pub fn with_claims<T, S: Into<String>>(mut request: Request<T>, sub: S) -> Request<T> {
    request.extensions_mut().insert(test_claims(sub.into()));
    request
}

pub async fn insert_fake_user(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query!(
        r#"insert into users (google_sub) values ('testing') on conflict (google_sub) do nothing"#
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_database() -> TestDatabase {
    let config = Config::load(None).unwrap();
    TestDatabase::new(String::from("accountcat-testing-"), config.database)
        .await
        .unwrap()
}

use sqlx::PgPool;
use tonic::{Request, async_trait};

use crate::{auth::IdClaimExtractor, jwtutils::Claims};

pub mod test_database;

pub struct DummyIdClaimExtractor {
    claims: Claims,
}

impl DummyIdClaimExtractor {
    pub fn new(google_sub: String) -> Self {
        Self {
            claims: Claims {
                iss: Default::default(),
                azp: Default::default(),
                aud: Default::default(),
                sub: google_sub,
                iat: Default::default(),
                exp: Default::default(),
                picture: Default::default(),
                given_name: Default::default(),
                family_name: Default::default(),
                name: Default::default(),
            },
        }
    }
}

#[async_trait]
impl IdClaimExtractor for DummyIdClaimExtractor {
    async fn get_claims<T: Send + Sync>(&self, _request: &Request<T>) -> tonic::Result<Claims> {
        Ok(self.claims.clone())
    }
}

pub async fn insert_fake_user(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query!(
        r#"insert into users (google_sub) values ('testing') on conflict (google_sub) do nothing"#
    )
    .execute(pool)
    .await?;
    Ok(())
}

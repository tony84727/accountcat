use tonic::{Request, async_trait};
use tower_sessions::Session;

use crate::{jwtutils::Claims, server::SESSION_KEY_CLAIMS};

const NOT_LOGIN: &str = "please login first";

async fn get_claims<M>(request: &Request<M>) -> tonic::Result<Claims> {
    let session: Option<&Session> = request.extensions().get();
    let Some(session) = session else {
        return Err(tonic::Status::unauthenticated(NOT_LOGIN));
    };
    match session.get::<Claims>(SESSION_KEY_CLAIMS).await {
        Ok(Some(claims)) => Ok(claims),
        Ok(None) => Err(tonic::Status::unauthenticated(NOT_LOGIN)),
        Err(_err) => Err(tonic::Status::internal(String::new())),
    }
}

#[async_trait]
pub trait IdClaimExtractor {
    async fn get_claims<T: Send + Sync>(&self, request: &Request<T>) -> tonic::Result<Claims>;
}

pub struct FromSession;

#[async_trait]
impl IdClaimExtractor for FromSession {
    async fn get_claims<T: Send + Sync>(&self, request: &Request<T>) -> tonic::Result<Claims> {
        get_claims(request).await
    }
}

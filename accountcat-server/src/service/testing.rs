use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use rand::Rng;
use tonic::{Request, Response};
use tower_sessions::Session;
use tracing::error;

use crate::{
    idl::testing::testing_server::Testing,
    jwtutils::Claims,
    server::{SESSION_KEY_CLAIMS, ServerState},
    service::user::create_new_user,
};
use uuid::Uuid;

pub struct TestingApi {
    state: Arc<ServerState>,
}

impl TestingApi {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl Testing for TestingApi {
    async fn request_session(&self, request: Request<()>) -> tonic::Result<Response<()>> {
        let session: Option<&Session> = request.extensions().get();
        let Some(session) = session else {
            return Err(tonic::Status::internal(String::new()));
        };
        let claims = random_testing_claims();
        if let Err(err) = create_new_user(&self.state.database, &claims.sub).await {
            error!(action = "insert new user record for randomly generated session", err = ?err);
            return Err(tonic::Status::internal(String::new()));
        }
        if let Err(err) = session.insert(SESSION_KEY_CLAIMS, claims).await {
            error!(action = "create testing session", err = ?err);
            return Err(tonic::Status::internal(String::new()));
        }
        Ok(Response::new(()))
    }
}

fn random_testing_claims() -> Claims {
    let unix_pooch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let iat = unix_pooch.as_secs();
    let exp = iat + 3600;
    let name = Uuid::new_v4();
    let mut rng = rand::rng();
    let sub: i64 = rng.random_range(0..99999999999);
    Claims {
        iss: String::new(),
        azp: String::new(),
        aud: String::new(),
        sub: sub.to_string(),
        iat,
        exp,
        picture: String::new(),
        given_name: name.to_string(),
        family_name: String::from("TestingUser"),
        name: name.to_string(),
    }
}

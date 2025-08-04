use std::sync::Arc;

use tonic::{Request, Response};
use tower_sessions::Session;

use crate::{
    idl::user::{LoginRequest, Profile, user_server::User},
    jwtutils::Claims,
    server::{SESSION_KEY_CLAIMS, ServerState},
};

pub struct UserApi {
    state: Arc<ServerState>,
}

impl UserApi {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl User for UserApi {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> tonic::Result<Response<Profile>, tonic::Status> {
        let session: Option<&Session> = request.extensions().get();
        let Some(session) = session else {
            return Err(tonic::Status::internal(String::new()));
        };
        let current_subject = session
            .get::<Claims>(SESSION_KEY_CLAIMS)
            .await
            .map_err(|_| tonic::Status::internal(String::new()))?;
        if let Some(subject) = current_subject {
            return Ok(tonic::Response::new(Profile {
                name: Some(subject.name),
            }));
        }
        let claims = self
            .state
            .jwt_verify
            .verify(&request.get_ref().token)
            .map_err(|_| tonic::Status::unauthenticated("invalid token"))?;
        sqlx::query!(
            "insert into users (google_sub) values ($1) on conflict (google_sub) do nothing;",
            claims.sub
        )
        .execute(&self.state.database)
        .await
        .map_err(|_| tonic::Status::internal(String::new()))?;
        session
            .insert(SESSION_KEY_CLAIMS, &claims)
            .await
            .map_err(|_| tonic::Status::internal(String::new()))?;
        Ok(tonic::Response::new(Profile {
            name: Some(claims.name),
        }))
    }
    async fn get_name(
        &self,
        request: Request<()>,
    ) -> tonic::Result<tonic::Response<Profile>, tonic::Status> {
        let session: Option<&Session> = request.extensions().get();
        let Some(session) = session else {
            return Err(tonic::Status::internal(String::new()));
        };
        let Some(claims) = session
            .get::<Claims>(SESSION_KEY_CLAIMS)
            .await
            .map_err(|_| tonic::Status::internal(String::new()))?
        else {
            return Ok(tonic::Response::new(Profile::default()));
        };

        Ok(tonic::Response::new(Profile {
            name: Some(claims.name),
        }))
    }
}

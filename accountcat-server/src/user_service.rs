use std::{collections::HashSet, sync::Arc};

use secrecy::{ExposeSecret, SecretString};
use tonic::{Request, Response};
use tower_sessions::Session;

use crate::{
    idl::user::{LoginRequest, Param, Profile, user_server::User},
    jwtutils::Claims,
    server::{SESSION_KEY_CLAIMS, ServerState},
};

pub struct UserApi {
    state: Arc<ServerState>,
    google_client_id: SecretString,
    administrators: Arc<HashSet<String>>,
}

impl UserApi {
    pub fn new(
        state: Arc<ServerState>,
        google_client_id: SecretString,
        administrators: Arc<HashSet<String>>,
    ) -> Self {
        Self {
            state,
            google_client_id,
            administrators,
        }
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
                name: subject.name,
                is_admin: self.administrators.contains(&subject.sub),
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
            name: claims.name,
            is_admin: self.administrators.contains(&claims.sub),
        }))
    }
    async fn get_profile(
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
            name: claims.name,
            is_admin: self.administrators.contains(&claims.sub),
        }))
    }
    async fn get_param(&self, _request: Request<()>) -> tonic::Result<tonic::Response<Param>> {
        Ok(tonic::Response::new(Param {
            google_client_id: self.google_client_id.expose_secret().into(),
        }))
    }
}

use std::sync::Arc;

use axum::Router;
use tonic::Request;
use tonic_web::GrpcWebLayer;
use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

use crate::{
    config,
    idl::{LoginRequest, Profile, user_server::UserServer},
    jwtutils::{self, Claims, JwtVerifier},
};

const SESSION_KEY_CLAIMS: &str = "claims";

struct ServerState {
    jwt_verify: JwtVerifier,
}

async fn init_state() -> ServerState {
    let server_config = config::load().expect("load server config");
    let verifier = JwtVerifier::new(jwtutils::DEFAULT_JWK_URL, server_config.login.client_id)
        .await
        .expect("init jwt verifier");
    ServerState {
        jwt_verify: verifier,
    }
}

struct UserApi {
    server_status: Arc<ServerState>,
}

#[tonic::async_trait]
impl crate::idl::user_server::User for UserApi {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> tonic::Result<tonic::Response<Profile>, tonic::Status> {
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
            .server_status
            .jwt_verify
            .verify(&request.get_ref().token)
            .map_err(|_| tonic::Status::unauthenticated("invalid token"))?;
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

pub async fn main() {
    tracing_subscriber::fmt::init();
    let serve_ui = ServeDir::new("ui/dist");
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store);
    let server_state = Arc::new(init_state().await);
    let user_api = UserServer::new(UserApi {
        server_status: server_state.clone(),
    });
    let mut grpc_server_builder = tonic::service::Routes::builder();
    grpc_server_builder.add_service(user_api);
    let grpc_server = grpc_server_builder.routes();

    let app = Router::new()
        .nest(
            "/api",
            grpc_server
                .into_axum_router()
                .layer(GrpcWebLayer::new())
                .layer(session_layer),
        )
        .fallback_service(serve_ui);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

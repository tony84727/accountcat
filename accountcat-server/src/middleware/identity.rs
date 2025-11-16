use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use tower_sessions::Session;
use tracing::error;

use crate::{auth::NOT_LOGIN, jwtutils::Claims, server::SESSION_KEY_CLAIMS};

const USER_SERVICE_PREFIX: &str = "/accountcat.user.User/";

pub async fn enforce_identity(mut request: Request, next: Next) -> Response {
    if should_skip(&request) {
        return next.run(request).await;
    }

    match attach_claims(&mut request).await {
        Ok(()) => next.run(request).await,
        Err(status) => grpc_error_response(status),
    }
}

fn should_skip(request: &Request) -> bool {
    request.uri().path().starts_with(USER_SERVICE_PREFIX)
}

async fn attach_claims(request: &mut Request) -> Result<(), tonic::Status> {
    let session: Option<&Session> = request.extensions().get();
    let Some(session) = session else {
        return Err(tonic::Status::unauthenticated(NOT_LOGIN));
    };

    match session.get::<Claims>(SESSION_KEY_CLAIMS).await {
        Ok(Some(claims)) => {
            request.extensions_mut().insert(claims);
            Ok(())
        }
        Ok(None) => Err(tonic::Status::unauthenticated(NOT_LOGIN)),
        Err(err) => {
            error!(
                service = "identity_middleware",
                action = "load_session_claims",
                error = ?err
            );
            Err(tonic::Status::internal(String::new()))
        }
    }
}

fn grpc_error_response(status: tonic::Status) -> Response {
    let mut builder = http::Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/grpc"),
        )
        .header("grpc-status", status.code().to_string());

    if !status.message().is_empty() {
        let encoded = percent_encode(status.message().as_bytes(), NON_ALPHANUMERIC).to_string();
        if let Ok(value) = HeaderValue::from_str(&encoded) {
            builder = builder.header("grpc-message", value);
        }
    }

    builder
        .body(Body::empty())
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

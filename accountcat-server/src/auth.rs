use tonic::{Request, Status};

use crate::jwtutils::Claims;

pub const NOT_LOGIN: &str = "please login first";

pub fn claims_from_request<T>(request: &Request<T>) -> Result<Claims, Status> {
    request
        .extensions()
        .get::<Claims>()
        .cloned()
        .ok_or_else(|| Status::unauthenticated(NOT_LOGIN))
}

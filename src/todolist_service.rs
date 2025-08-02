use std::sync::Arc;

use prost_types::Timestamp;
use tonic::{Request, Response};
use tower_sessions::Session;

use crate::{
    idl::todolist::{ListResult, NewTask, Task, todolist_server::Todolist},
    jwtutils::Claims,
    server::{SESSION_KEY_CLAIMS, ServerState},
};

pub struct TodolistApi {
    state: Arc<ServerState>,
}

impl TodolistApi {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
    }
}

const NOT_LOGIN: &str = "please login first";

#[tonic::async_trait]
impl Todolist for TodolistApi {
    async fn list(&self, request: Request<()>) -> tonic::Result<Response<ListResult>> {
        let session: Option<&Session> = request.extensions().get();
        let Some(session) = session else {
            return Err(tonic::Status::unauthenticated(NOT_LOGIN));
        };
        let claims = match session.get::<Claims>(SESSION_KEY_CLAIMS).await {
            Ok(Some(claims)) => claims,
            Ok(None) => {
                return Err(tonic::Status::unauthenticated(NOT_LOGIN));
            }
            Err(_err) => {
                return Err(tonic::Status::internal(String::new()));
            }
        };
        let tasks = match sqlx::query!(
            "select todo_tasks.id, todo_tasks.name, todo_tasks.description, todo_tasks.completed, todo_tasks.created_at
from todo_tasks
join users on users.id = todo_tasks.user_id
where google_sub = $1",
            claims.sub
        )
        .map(|x| Task {
            id: x.id.to_string(),
            name: x.name.unwrap_or_default(),
            completed: x.completed,
            description: x.description,
            created_at: x.created_at.map(|x| Timestamp {
                seconds: x.unix_timestamp(),
                nanos: x.nanosecond() as i32,
            }),
        })
        .fetch_all(&self.state.database)
        .await
        {
            Ok(tasks) => tasks,
            Err(_err) => {
                return Err(tonic::Status::internal(String::new()));
            }
        };
        let message = ListResult { tasks };
        Ok(Response::new(message))
    }

    async fn add(&self, request: Request<NewTask>) -> tonic::Result<Response<()>> {
        let session: Option<&Session> = request.extensions().get();
        let Some(session) = session else {
            return Err(tonic::Status::unauthenticated(NOT_LOGIN));
        };
        let claims = match session.get::<Claims>(SESSION_KEY_CLAIMS).await {
            Ok(Some(claims)) => claims,
            Ok(None) => {
                return Err(tonic::Status::unauthenticated(NOT_LOGIN));
            }
            Err(_err) => {
                return Err(tonic::Status::internal(String::new()));
            }
        };
        let NewTask { name, description } = request.into_inner();
        if sqlx::query!(
            "insert into todo_tasks (user_id, name, description)
select users.id, $1, $2
from users
where google_sub = $3",
            name,
            description,
            claims.sub
        )
        .execute(&self.state.database)
        .await
        .is_err()
        {
            return Err(tonic::Status::internal(String::new()));
        };
        Ok(Response::new(()))
    }
}

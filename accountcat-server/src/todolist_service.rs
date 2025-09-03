use std::sync::Arc;

use tonic::{Request, Response};

use crate::{
    auth::IdClaimExtractor,
    idl::todolist::{ListResult, NewTask, Task, TaskUpdate, todolist_server::Todolist},
    protobufutils::to_proto_timestamp,
    server::ServerState,
};

pub struct TodolistApi<I> {
    state: Arc<ServerState>,
    id_claim_extractor: Arc<I>,
}

impl<I> TodolistApi<I> {
    pub fn new(state: Arc<ServerState>, id_claim_extractor: Arc<I>) -> Self {
        Self {
            state,
            id_claim_extractor,
        }
    }
}

#[tonic::async_trait]
impl<I> Todolist for TodolistApi<I>
where
    I: IdClaimExtractor + Send + Sync + 'static,
{
    async fn list(&self, request: Request<()>) -> tonic::Result<Response<ListResult>> {
        let claims = self.id_claim_extractor.get_claims(&request).await?;
        let tasks = match sqlx::query!(
            "select todo_tasks.id, todo_tasks.name, todo_tasks.description, todo_tasks.completed, todo_tasks.created_at
from todo_tasks
join users on users.id = todo_tasks.user_id
where google_sub = $1
order by todo_tasks.created_at desc",
            claims.sub
        )
        .map(|x| Task {
            id: x.id.to_string(),
            name: x.name.unwrap_or_default(),
            completed: x.completed,
            description: x.description,
            created_at: x.created_at.map(to_proto_timestamp),
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
        let claims = self.id_claim_extractor.get_claims(&request).await?;
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

    async fn update_task(&self, request: Request<TaskUpdate>) -> tonic::Result<Response<Task>> {
        let claims = self.id_claim_extractor.get_claims(&request).await?;
        let TaskUpdate { id, completed } = request.into_inner();
        let Ok(id) = id.parse::<i32>() else {
            return Err(tonic::Status::not_found(String::new()));
        };
        let Ok(task) = sqlx::query!(
            "update todo_tasks
set completed = coalesce($3, todo_tasks.completed)
from users
where todo_tasks.id = $1 and todo_tasks.user_id = users.id and users.google_sub = $2
returning todo_tasks.*",
            id,
            claims.sub,
            completed
        )
        .fetch_one(&self.state.database)
        .await
        else {
            return Err(tonic::Status::internal(String::new()));
        };
        Ok(Response::new(Task {
            id: task.id.to_string(),
            name: task.name.unwrap_or_default(),
            completed: task.completed,
            description: task.description,
            created_at: task.created_at.map(to_proto_timestamp),
        }))
    }
}

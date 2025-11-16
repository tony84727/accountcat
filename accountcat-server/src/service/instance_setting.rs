use std::{collections::HashSet, sync::Arc};

use tonic::{Request, Response, Status};
use tracing::error;

use crate::{
    auth::claims_from_request,
    idl::instance_setting::{Announcement, instance_setting_server::InstanceSetting},
    server::ServerState,
};

pub struct InstanceSettingApi {
    state: Arc<ServerState>,
    administrators: Arc<HashSet<String>>,
}

impl InstanceSettingApi {
    pub fn new(state: Arc<ServerState>, administrators: Arc<HashSet<String>>) -> Self {
        Self {
            state,
            administrators,
        }
    }
}

#[tonic::async_trait]
impl InstanceSetting for InstanceSettingApi {
    async fn set_announcement(
        &self,
        request: Request<Announcement>,
    ) -> tonic::Result<Response<()>> {
        let claims = claims_from_request(&request)?;
        if !self.administrators.contains(&claims.sub) {
            return Err(Status::permission_denied("you're not an admin"));
        }
        let Ok(mut tx) = self.state.database.begin().await else {
            return Err(Status::internal(String::new()));
        };
        if let Err(err) = sqlx::query!(
            "update announcements set hidden_at = now()
where id = (select id from announcements where hidden_at is null order by created_at desc limit 1)"
        )
        .execute(&mut *tx)
        .await
        {
            error!(action = "set announcement", error = ?err);
            return Err(Status::internal(String::new()));
        }
        let Announcement { content } = request.into_inner();
        if let Err(err) = sqlx::query!("insert into announcements (content) values ($1)", content)
            .execute(&mut *tx)
            .await
        {
            error!(action = "set announcement", error = ?err);
            return Err(Status::internal(String::new()));
        }
        match tx.commit().await {
            Ok(()) => Ok(Response::new(())),
            Err(err) => {
                error!(action = "set announcement", error = ?err);
                Err(Status::internal(String::new()))
            }
        }
    }

    async fn revoke_announcement(&self, request: Request<()>) -> tonic::Result<Response<()>> {
        let claims = claims_from_request(&request)?;
        if !self.administrators.contains(&claims.sub) {
            return Err(Status::permission_denied("you're not an admin"));
        }
        match sqlx::query!("update announcements set hidden_at = now() where hidden_at is null")
            .execute(&self.state.database)
            .await
        {
            Ok(_) => Ok(Response::new(())),
            Err(err) => {
                error!(action = "revoke announcement", error = ?err);
                Err(Status::internal(String::new()))
            }
        }
    }
}

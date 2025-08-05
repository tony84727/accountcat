use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::error;

use crate::{
    auth::get_claims,
    idl::accounting::{Item, ItemList, accounting_server::Accounting},
    protobufutils::to_proto_timestamp,
    server::ServerState,
};

pub struct AccountingApi {
    state: Arc<ServerState>,
}

impl AccountingApi {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl Accounting for AccountingApi {
    async fn list(&self, request: Request<()>) -> tonic::Result<Response<ItemList>> {
        let claims = get_claims(&request).await?;
        let items = match sqlx::query!("select accounting_items.name, accounting_items.income, accounting_items.expense, accounting_items.created_at
from accounting_items
join users on users.id = accounting_items.user_id
where users.google_sub = $1", claims.sub)
            .map(|x| Item {income:x.income.to_string(), name: x.name.unwrap_or_default(), expense: x.expense.to_string(), created_at: x.created_at.map(to_proto_timestamp)})
            .fetch_all(&self.state.database)
            .await {
            Ok(x) => x,
            Err(err) => {
                error!(service = "accounting", type = "query_error", message = err.to_string());
                return Err(Status::internal(String::new()));
            }
        };
        Ok(Response::new(ItemList { items }))
    }
}

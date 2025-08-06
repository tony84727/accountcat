use std::sync::Arc;

use sqlx::types::BigDecimal;
use tonic::{Request, Response, Status};
use tracing::error;

use crate::{
    auth::get_claims,
    idl::accounting::{Item, ItemList, NewItem, accounting_server::Accounting},
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
        let items = match sqlx::query!("select accounting_items.id, accounting_items.name, accounting_items.income, accounting_items.expense, accounting_items.created_at
from accounting_items
join users on users.id = accounting_items.user_id
where users.google_sub = $1
order by accounting_items.created_at desc", claims.sub)
            .map(|x| Item {
                id: x.id.to_string(),
                income:x.income.to_string(),
                name: x.name.unwrap_or_default(),
                expense: x.expense.to_string(),
                created_at: x.created_at.map(to_proto_timestamp)
            })
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
    async fn add(&self, request: Request<NewItem>) -> tonic::Result<Response<Item>> {
        let claims = get_claims(&request).await?;
        let NewItem {
            name,
            income,
            expense,
        } = request.into_inner();
        let Ok(income) = income.parse::<BigDecimal>() else {
            return Err(Status::invalid_argument("income isn't numeric"));
        };
        let Ok(expense) = expense.parse::<BigDecimal>() else {
            return Err(Status::invalid_argument("expense isn't numeric"));
        };
        match sqlx::query!("insert into accounting_items (user_id, name, income, expense)
select users.id, $1, $2, $3
from users
where users.google_sub = $4
returning accounting_items.id, accounting_items.name, accounting_items.income, accounting_items.expense, accounting_items.created_at", name, income, expense, claims.sub).fetch_one(&self.state.database)
            .await {
            Ok(record) => Ok(Response::new(Item {
                id: record.id.to_string(),
                name: record.name.unwrap_or_default(),
                income: record.income.to_string(),
                expense: record.expense.to_string(),
                created_at: record.created_at.map(to_proto_timestamp)
            })),
                Err(err) => {
                Err(Status::internal(String::new()))
            },
        }
    }
}

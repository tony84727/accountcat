use std::sync::Arc;

use iso_currency::{Currency, IntoEnumIterator};
use sqlx::types::BigDecimal;
use tonic::{Request, Response, Status};
use tracing::error;

use crate::{
    auth::get_claims,
    idl::accounting::{
        CurrencyList, Item, ItemList, NewItem, NewTag, Tag, TagList, TagSearch,
        accounting_server::Accounting,
    },
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
            tags,
        } = request.into_inner();
        let Ok(income) = income.parse::<BigDecimal>() else {
            return Err(Status::invalid_argument("income isn't numeric"));
        };
        let Ok(expense) = expense.parse::<BigDecimal>() else {
            return Err(Status::invalid_argument("expense isn't numeric"));
        };
        let Ok(mut tx) = self.state.database.begin().await else {
            return Err(Status::internal(String::new()));
        };
        let item = match sqlx::query!("insert into accounting_items (user_id, name, income, expense)
select users.id, $1, $2, $3
from users
where users.google_sub = $4
returning accounting_items.id, accounting_items.name, accounting_items.income, accounting_items.expense, accounting_items.created_at", name, income, expense, claims.sub)
            .fetch_one(&mut *tx)
            .await {
            Ok(record) => Ok(record),
                Err(_err) => {
                Err(Status::internal(String::new()))
            },
        }?;
        let tag_id: Vec<i32> = tags.iter().filter_map(|x| x.parse().ok()).collect();
        sqlx::query!(
            "insert into accounting_item_tags (tag_id, accounting_item_id)
select tags.id, $1
from tags
join users on users.id = tags.user_id
where users.google_sub = $2 and tags.id = any($3)",
            item.id,
            claims.sub,
            &tag_id[..],
        )
        .execute(&mut *tx)
        .await
        .map_err(|_err| Status::internal(String::new()))?;
        tx.commit()
            .await
            .map_err(|_err| Status::internal(String::new()))?;
        Ok(Response::new(Item {
            id: item.id.to_string(),
            name: item.name.unwrap_or_default(),
            income: item.income.to_string(),
            expense: item.expense.to_string(),
            created_at: item.created_at.map(to_proto_timestamp),
        }))
    }
    async fn complete_tag(&self, request: Request<TagSearch>) -> tonic::Result<Response<TagList>> {
        let claims = get_claims(&request).await?;
        let TagSearch { keyword } = request.into_inner();
        match sqlx::query!(
            r#"select tags.id, tags.name
from tags
join users on users.id = tags.user_id
where users.google_sub = $1 and tags.name like $2 escape '\'"#,
            claims.sub,
            format!("%{}%", keyword.replace("%", "\\%"))
        )
        .map(|r| Tag {
            id: r.id.to_string(),
            name: r.name,
        })
        .fetch_all(&self.state.database)
        .await
        {
            Ok(tags) => Ok(Response::new(TagList { tags })),
            Err(_err) => Err(Status::internal(String::new())),
        }
    }

    async fn create_tag(&self, request: Request<NewTag>) -> tonic::Result<Response<Tag>> {
        let claims = get_claims(&request).await?;
        let NewTag { name } = request.into_inner();
        match sqlx::query!(
            "insert into tags (user_id, name)
select users.id, $1
from users
where users.google_sub = $2
returning tags.id, tags.name",
            name,
            claims.sub
        )
        .fetch_one(&self.state.database)
        .await
        {
            Ok(record) => Ok(Response::new(Tag {
                id: record.id.to_string(),
                name: record.name,
            })),
            Err(_err) => Err(Status::internal(String::new())),
        }
    }

    async fn list_currency(&self, _request: Request<()>) -> tonic::Result<Response<CurrencyList>> {
        let code = Currency::iter()
            .filter(|x| x.flags().is_empty())
            .map(|x| String::from(x.code()))
            .collect();
        Ok(Response::new(CurrencyList { code }))
    }
}

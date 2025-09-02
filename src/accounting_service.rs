use std::sync::Arc;

use hash_ids::HashIds;
use iso_currency::{Currency, IntoEnumIterator};
use secrecy::{ExposeSecret, SecretString};
use sqlx::types::BigDecimal;
use tonic::{Request, Response, Status};
use tracing::error;

use crate::{
    auth::IdClaimExtractor,
    idl::accounting::{
        Amount, AmountType, CurrencyList, DeleteItem, Item, ItemList, NewItem, NewTag, Tag,
        TagList, TagSearch, UpdateItemRequest, accounting_server::Accounting,
    },
    protobufutils::{from_proto_timestamp, to_proto_timestamp},
    server::ServerState,
};

pub struct AccountingApi<I> {
    state: Arc<ServerState>,
    id_claim_extractor: Arc<I>,
    hashids: HashIds,
}

impl<I> AccountingApi<I> {
    pub fn new(state: Arc<ServerState>, id_claim_extractor: Arc<I>, salt: SecretString) -> Self {
        let hashids = HashIds::builder().with_salt(salt.expose_secret()).finish();
        Self {
            state,
            id_claim_extractor,
            hashids,
        }
    }

    fn encode_id(&self, id: i32) -> String {
        self.hashids.encode(&[id as u64])
    }

    fn decode_id(&self, id: &str) -> Option<i32> {
        let numbers = self.hashids.decode(id).ok()?;
        numbers.first().and_then(|&n| i32::try_from(n).ok())
    }
}

#[tonic::async_trait]
impl<I> Accounting for AccountingApi<I>
where
    I: IdClaimExtractor + Send + Sync + 'static,
{
    async fn list(&self, request: Request<()>) -> tonic::Result<Response<ItemList>> {
        let claims = self.id_claim_extractor.get_claims(&request).await?;
        let items = match sqlx::query!("select accounting_items.id, accounting_items.name, accounting_items.amount, accounting_items.currency, accounting_items.created_at, accounting_items.occurred_at
from accounting_items
join users on users.id = accounting_items.user_id
where users.google_sub = $1
order by accounting_items.created_at desc", claims.sub)
            .map(|x| Item {
                id: self.encode_id(x.id),
                amount: Some(Amount{
                    amount: x.amount.normalized().to_string(),
                    currency: x.currency,
                }),
                r#type: if x.amount < BigDecimal::from(0) {
                    AmountType::Expense
                } else {
                    AmountType::Income
                }.into(),
                name: x.name.unwrap_or_default(),
                created_at: x.created_at.map(to_proto_timestamp),
                occurred_at: Some(to_proto_timestamp(x.occurred_at)),
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
        let claims = self.id_claim_extractor.get_claims(&request).await?;
        let NewItem {
            name,
            amount,
            tags,
            r#type,
        } = request.into_inner();
        let Some(Amount { amount, currency }) = amount else {
            return Err(Status::invalid_argument("missing amount"));
        };
        let Ok(mut amount) = amount.parse::<BigDecimal>() else {
            return Err(Status::invalid_argument("amount isn't numeric"));
        };
        let Ok(mut tx) = self.state.database.begin().await else {
            return Err(Status::internal(String::new()));
        };
        if r#type == (AmountType::Expense as i32) {
            amount = -amount;
        }
        let item = match sqlx::query!(
            "insert into accounting_items (user_id, name, amount, currency)
select users.id, $1, $2, $3
from users
where users.google_sub = $4
returning accounting_items.id,
          accounting_items.name,
          accounting_items.amount,
          accounting_items.currency,
          accounting_items.created_at,
          accounting_items.occurred_at",
            name,
            amount,
            currency,
            claims.sub
        )
        .fetch_one(&mut *tx)
        .await
        {
            Ok(record) => Ok(record),
            Err(_err) => Err(Status::internal(String::new())),
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
            id: self.encode_id(item.id),
            name: item.name.unwrap_or_default(),
            amount: Some(Amount {
                currency: item.currency,
                amount: item.amount.normalized().to_plain_string(),
            }),
            r#type: if item.amount < BigDecimal::from(0) {
                AmountType::Expense
            } else {
                AmountType::Income
            }
            .into(),
            created_at: item.created_at.map(to_proto_timestamp),
            occurred_at: Some(to_proto_timestamp(item.occurred_at)),
        }))
    }
    async fn complete_tag(&self, request: Request<TagSearch>) -> tonic::Result<Response<TagList>> {
        let claims = self.id_claim_extractor.get_claims(&request).await?;
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
        let claims = self.id_claim_extractor.get_claims(&request).await?;
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

    async fn delete(&self, request: Request<DeleteItem>) -> tonic::Result<Response<()>> {
        let claims = self.id_claim_extractor.get_claims(&request).await?;
        let DeleteItem { id } = request.into_inner();
        let Some(id) = self.decode_id(&id) else {
            return Ok(Response::new(()));
        };
        if let Err(err) = sqlx::query!(
            r#"delete from accounting_items
using users
where users.google_sub = $1 and accounting_items.id = $2 and accounting_items.user_id = users.id"#,
            claims.sub,
            id,
        )
        .execute(&self.state.database)
        .await
        {
            error!(action = "delete accounting item", error = ?err);
            return Err(Status::internal(String::new()));
        }
        Ok(Response::new(()))
    }

    async fn update_item(
        &self,
        request: Request<UpdateItemRequest>,
    ) -> tonic::Result<Response<()>> {
        let claims = self.id_claim_extractor.get_claims(&request).await?;
        let UpdateItemRequest {
            id,
            name,
            amount,
            occurred_at,
        } = request.into_inner();
        let Some(id) = self.decode_id(&id) else {
            return Err(Status::invalid_argument("bad id"));
        };
        let (amount, currency) = match amount {
            Some(Amount { currency, amount }) => (Some(amount), Some(currency)),
            None => (None, None),
        };
        let amount = amount.and_then(|x| x.parse::<BigDecimal>().ok());
        if let Err(err) = sqlx::query!(
            "update accounting_items
set name = coalesce($1, name),
    occurred_at = coalesce($2, occurred_at),
    amount = coalesce((case when amount = 0 then 1 else sign(amount) end)*$3, amount),
    currency = coalesce($4, currency)
from users
where accounting_items.id = $5 and accounting_items.user_id = users.id and users.google_sub = $6",
            name,
            occurred_at.and_then(|x| from_proto_timestamp(x).ok()),
            amount,
            currency,
            id,
            claims.sub
        )
        .execute(&self.state.database)
        .await
        {
            error!(action = "update accounting item", error = ?err);
            return Err(Status::internal(String::new()));
        }
        Ok(Response::new(()))
    }
}

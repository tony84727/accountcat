create table accounting_item_tags (
  id serial primary key,
  tag_id integer not null references tags(id),
  accounting_item_id integer not null references accounting_items(id),
  created_at timestamp with time zone not null default now()
);

create index accounting_item_tags_accounting_item_id on accounting_item_tags(accounting_item_id);

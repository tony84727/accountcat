create table accounting_items (
  id serial primary key,
  user_id integer not null references users(id),
  name varchar(1024),
  income numeric(4) not null default 0,
  expense numeric(4) not null default 0,
  created_at timestamp with time zone default now()
);

create index accounting_items_user_id on accounting_items(user_id);
create index accounting_items_created_at on accounting_items(created_at);

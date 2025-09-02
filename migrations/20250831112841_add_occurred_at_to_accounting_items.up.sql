begin;
alter table accounting_items add column occurred_at timestamp with time zone not null default now();
update accounting_items set occurred_at = created_at;
commit;

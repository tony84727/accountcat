begin;
alter table accounting_item_tags drop constraint accounting_item_tags_accounting_item_id_fkey;
alter table accounting_item_tags add foreign key (accounting_item_id) references accounting_items(id);
commit;

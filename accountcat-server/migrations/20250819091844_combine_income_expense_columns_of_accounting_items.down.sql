begin;
alter table accounting_items rename column amount to income;
alter table accounting_items add column expense numeric(4) not null default 0;
update accounting_items set expense = -income, income = 0
where income < 0;
commit;

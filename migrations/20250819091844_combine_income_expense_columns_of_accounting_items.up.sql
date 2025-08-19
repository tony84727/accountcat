begin;
update accounting_items set income = -expense where income = 0 and expense != 0;
insert into accounting_items (user_id, name, income, expense, created_at)
select user_id, name, -expense, 0, created_at
from accounting_items
where accounting_items.income > 0 and accounting_items.expense != 0;
alter table accounting_items rename column income to amount;
alter table accounting_items drop expense;
commit;

create table todo_tasks (
  id serial primary key,
  user_id integer not null references users(id),
  name varchar(1024),
  description varchar(1024),
  completed boolean not null default false,
  created_at timestamp with time zone default now()
);

create index todo_tasks_created_at on todo_tasks(created_at);

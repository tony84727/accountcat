create table announcements (
  id serial primary key,
  content varchar(200) not null,
  hidden_at timestamp with time zone null,
  created_at timestamp with time zone not null default now()
);

create index announcements_created_at on announcements(created_at);

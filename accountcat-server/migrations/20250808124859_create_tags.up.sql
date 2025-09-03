create table tags (
  id serial primary key,
  user_id integer references users(id),
  name varchar(255) not null,
  created_at timestamp with time zone not null default now(),
  constraint tags_name_per_user unique(user_id, name)
);

create index tags_user_id on tags(user_id);

create table users (
  id serial primary key,
  google_sub varchar(255) unique,
  created_at timestamp with time zone default now()
);

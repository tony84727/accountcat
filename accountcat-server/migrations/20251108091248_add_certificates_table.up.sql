create table certificates (
    id serial primary key, 
    serial decimal(50,0) not null,
    dn bytea null,
    country varchar(40) null,
    state varchar(200) null,
    locality varchar(200) null,
    organization varchar(200) null,
    organizational_unit varchar(200) null,
    common_name varchar(200) null,
    not_before timestamp with time zone not null,
    not_after timestamp with time zone not null,
    created_at timestamp with time zone not null default now()
);

create index certificates_not_after on certificates(not_after);
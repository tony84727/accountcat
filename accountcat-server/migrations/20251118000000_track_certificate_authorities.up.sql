alter table certificates
    add column der bytea null,
    add column private_key_der bytea null,
    add column issuer_certificate_id integer null,
    add column is_ca boolean not null default false,
    add column trusted boolean not null default false;

alter table certificates
    add constraint certificates_issuer_certificate_id_fkey
    foreign key (issuer_certificate_id) references certificates(id) on delete restrict;

create index certificates_issuer_certificate_id_serial on certificates(issuer_certificate_id, serial);
create index certificates_issuer_certificate_id on certificates(issuer_certificate_id);

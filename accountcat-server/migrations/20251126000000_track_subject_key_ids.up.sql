alter table certificates
    alter column serial drop not null,
    alter column not_before drop not null,
    alter column not_after drop not null,
    add column subject_key_id bytea null;

create unique index certificates_subject_key_id
    on certificates(subject_key_id)
    where subject_key_id is not null;

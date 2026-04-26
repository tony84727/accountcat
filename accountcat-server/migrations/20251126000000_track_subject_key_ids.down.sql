drop index if exists certificates_subject_key_id;

alter table certificates
    drop column if exists subject_key_id,
    alter column serial set not null,
    alter column not_before set not null,
    alter column not_after set not null;

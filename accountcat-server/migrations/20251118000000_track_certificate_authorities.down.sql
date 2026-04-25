drop index if exists certificates_issuer_certificate_id;
drop index if exists certificates_issuer_certificate_id_serial;

alter table certificates
    drop constraint if exists certificates_issuer_certificate_id_fkey;

alter table certificates
    drop column if exists trusted,
    drop column if exists is_ca,
    drop column if exists issuer_certificate_id,
    drop column if exists private_key_der,
    drop column if exists der;

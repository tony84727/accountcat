use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};

use rcgen::{
    BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose,
    PKCS_ED25519, PublicKeyData,
};
use rustls_pki_types::CertificateDer;
use sqlx::{Executor, PgPool, Postgres, Row, types::BigDecimal};
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use tonic::async_trait;
use x509_parser::{
    certificate::X509Certificate,
    certification_request::X509CertificationRequest,
    extensions::ParsedExtension,
    prelude::FromDer,
    x509::{AttributeTypeAndValue, X509Name},
};

use crate::pki::{
    crt::IssuedCertificate,
    csr::{CreateError, ToBeSignedCertificate},
};

pub(crate) fn create_option_for_sensitive_data() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true).mode(0o600);
    options
}

#[derive(Debug)]
pub struct CertificateAuthority {
    id: Option<i32>,
    keypair: KeyPair,
    certificate_der: Vec<u8>,
}

impl PartialEq for CertificateAuthority {
    fn eq(&self, other: &Self) -> bool {
        self.keypair.algorithm() == other.keypair.algorithm()
            && self.keypair.serialized_der() == other.keypair.serialized_der()
            && self.certificate_der == other.certificate_der
    }
}

impl Eq for CertificateAuthority {}

impl CertificateAuthority {
    pub fn generate() -> Result<Self, GenerateError> {
        let keypair = KeyPair::generate_for(&PKCS_ED25519)?;
        let mut params = CertificateParams::default();
        params.distinguished_name.push(DnType::CountryName, "TW");
        params
            .distinguished_name
            .push(DnType::OrganizationName, "Accountcat project");
        params
            .distinguished_name
            .push(DnType::CommonName, "Root CA");
        params.not_before = OffsetDateTime::now_utc();
        params.not_after = params.not_before.saturating_add(Duration::days(3650));
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
        ];
        let certificate = params.self_signed(&keypair)?;
        Ok(Self {
            id: None,
            keypair,
            certificate_der: certificate.der().to_vec(),
        })
    }

    pub async fn initialize(db: &PgPool) -> Result<Self, InitializeError> {
        let mut certificate_authority = Self::generate()?;
        let metadata = CertificateMetadata::parse(&certificate_authority.certificate_der)?;
        let certificate_id = insert_certificate(
            db,
            &metadata,
            true,
            true,
            None,
            &certificate_authority.certificate_der,
            Some(certificate_authority.keypair.serialized_der()),
        )
        .await?;
        certificate_authority.id = Some(certificate_id);
        Ok(certificate_authority)
    }

    pub async fn load(db: &PgPool) -> Result<Self, LoadError> {
        let row = sqlx::query(
            "select
                id,
                private_key_der,
                der as certificate_der
            from certificates
            where trusted
                and is_ca
                and der is not null
                and private_key_der is not null
            order by created_at desc, id desc
            limit 1",
        )
        .fetch_optional(db)
        .await?
        .ok_or(LoadError::MissingTrustedCa)?;
        Self::load_from_row(row)
    }

    pub async fn load_by_id(db: &PgPool, issuer_id: i32) -> Result<Self, LoadError> {
        let row = sqlx::query(
            "select
                id,
                trusted,
                is_ca,
                private_key_der,
                der as certificate_der
            from certificates
            where id = $1",
        )
        .bind(issuer_id)
        .fetch_optional(db)
        .await?
        .ok_or(LoadError::MissingIssuer(issuer_id))?;
        let trusted = row.try_get::<bool, _>("trusted")?;
        let is_ca = row.try_get::<bool, _>("is_ca")?;
        let certificate_der: Option<Vec<u8>> = row.try_get("certificate_der")?;
        let private_key_der: Option<Vec<u8>> = row.try_get("private_key_der")?;
        if !trusted || !is_ca || certificate_der.is_none() || private_key_der.is_none() {
            return Err(LoadError::IssuerNotUsable(issuer_id));
        }
        Self::load_from_parts(issuer_id, certificate_der, private_key_der)
    }

    pub fn certificate_der(&self) -> &[u8] {
        &self.certificate_der
    }

    pub fn issue_with_date(
        &self,
        subject: &str,
        not_before: OffsetDateTime,
        not_after: OffsetDateTime,
    ) -> Result<IssuedCertificate, IssueError> {
        let tbs = ToBeSignedCertificate::create(subject, not_before, not_after)?;
        let issuer = self.get_issuer();
        let certificate = tbs.signed_by(&issuer)?;
        Ok(certificate)
    }

    pub fn sign(&self, tbs: ToBeSignedCertificate) -> Result<IssuedCertificate, rcgen::Error> {
        let issuer = self.get_issuer();
        tbs.signed_by(&issuer)
    }

    fn get_issuer(&self) -> Issuer<'static, &KeyPair> {
        let cert_der = CertificateDer::from_slice(self.certificate_der.as_slice());
        Issuer::from_ca_cert_der(&cert_der, &self.keypair)
            .expect("stored CA certificate should remain valid")
    }

    fn load_from_row(row: sqlx::postgres::PgRow) -> Result<Self, LoadError> {
        let id = row.try_get("id")?;
        let certificate_der = row.try_get("certificate_der")?;
        let private_key_der = row.try_get("private_key_der")?;
        Self::load_from_parts(id, certificate_der, private_key_der)
    }

    fn load_from_parts(
        id: i32,
        certificate_der: Option<Vec<u8>>,
        private_key_der: Option<Vec<u8>>,
    ) -> Result<Self, LoadError> {
        let certificate_der = certificate_der.ok_or(LoadError::MissingStoredCertificateDer)?;
        let private_key_der = private_key_der.ok_or(LoadError::MissingStoredPrivateKey)?;
        let keypair = KeyPair::try_from(private_key_der)?;
        Ok(Self {
            id: Some(id),
            keypair,
            certificate_der,
        })
    }
}

#[async_trait]
impl CertificateIssuer for CertificateAuthority {
    fn issuer_certificate_id(&self) -> Option<i32> {
        self.id
    }

    async fn issue(
        &self,
        subject: &str,
        duration: Duration,
    ) -> Result<IssuedCertificate, IssueError> {
        self.issue_with_date(
            subject,
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc()
                .checked_add(duration)
                .ok_or(IssueError::InvalidNotBefore)?,
        )
    }
}

#[async_trait]
pub trait CertificateIssuer<E = IssueError> {
    fn issuer_certificate_id(&self) -> Option<i32> {
        None
    }

    async fn issue(&self, subject: &str, duration: Duration) -> Result<IssuedCertificate, E>;
}

pub struct TrackedCertificateIssuer<I: CertificateIssuer> {
    db: PgPool,
    issuer: I,
}

impl<I: CertificateIssuer> TrackedCertificateIssuer<I> {
    pub fn new(db: PgPool, issuer: I) -> Self {
        Self { db, issuer }
    }
}

#[async_trait]
impl<I: CertificateIssuer + Send + Sync> CertificateIssuer<TrackedIssueError>
    for TrackedCertificateIssuer<I>
{
    async fn issue(
        &self,
        subject: &str,
        duration: Duration,
    ) -> Result<IssuedCertificate, TrackedIssueError> {
        let issued = self.issuer.issue(subject, duration).await?;
        let metadata = CertificateMetadata::parse(issued.certificate.der())?;
        insert_certificate(
            &self.db,
            &metadata,
            false,
            false,
            self.issuer.issuer_certificate_id(),
            issued.certificate.der(),
            Some(issued.key.serialized_der()),
        )
        .await?;
        Ok(issued)
    }
}

pub struct PendingCertificateSigningRequest {
    pub id: i32,
    pub request_pem: String,
    pub request_der: Vec<u8>,
    pub private_key_der: Vec<u8>,
    pub subject_key_id: Vec<u8>,
}

pub async fn create_pending_csr(
    db: &PgPool,
    subject: &str,
    duration: Duration,
) -> Result<PendingCertificateSigningRequest, PendingCsrError> {
    let not_before = OffsetDateTime::now_utc();
    let not_after = not_before
        .checked_add(duration)
        .ok_or(PendingCsrError::InvalidNotAfter)?;
    let tbs = ToBeSignedCertificate::create(subject, not_before, not_after)?;
    let csr = tbs.serialize_request()?;
    let request_der = csr.der().as_ref().to_vec();
    let request_pem = csr.pem()?;
    let (_, parsed) =
        X509CertificationRequest::from_der(&request_der).map_err(PendingCsrError::InvalidCsr)?;
    let metadata = CertificateMetadata::parse_subject(&parsed.certification_request_info.subject);
    let subject_key_id = tbs.subject_key_id();
    let private_key_der = tbs.key.serialized_der().to_vec();
    let id = insert_pending_certificate(
        db,
        &metadata,
        &subject_key_id,
        Some(private_key_der.as_slice()),
    )
    .await?;
    Ok(PendingCertificateSigningRequest {
        id,
        request_pem,
        request_der,
        private_key_der,
        subject_key_id,
    })
}

#[derive(Debug)]
pub struct ImportedCertificate {
    pub id: i32,
    pub created: bool,
    pub idempotent: bool,
}

pub async fn import_certificate(
    db: &PgPool,
    certificate_der: &[u8],
) -> Result<ImportedCertificate, ImportCertificateError> {
    let metadata = CertificateMetadata::parse(certificate_der)?;
    let subject_key_id = metadata
        .subject_key_id
        .as_deref()
        .ok_or(ImportCertificateError::MissingSubjectKeyIdentifier)?;
    let issuer_certificate_id = match &metadata.authority_key_id {
        Some(authority_key_id) => find_certificate_by_subject_key_id(db, authority_key_id).await?,
        None => None,
    };

    if let Some(row) = sqlx::query!(
        "select id, der, private_key_der, trusted
        from certificates
        where subject_key_id = $1",
        subject_key_id,
    )
    .fetch_optional(db)
    .await?
    {
        let id = row.id;
        let existing_der = row.der;
        if existing_der.as_deref() == Some(certificate_der) {
            return Ok(ImportedCertificate {
                id,
                created: false,
                idempotent: true,
            });
        }
        if existing_der.is_some() {
            return Err(ImportCertificateError::CertificateAlreadyImported(id));
        }
        if let Some(private_key_der) = row.private_key_der {
            verify_certificate_matches_private_key(certificate_der, &private_key_der)?;
        }
        update_certificate(
            db,
            id,
            &metadata,
            metadata.is_ca,
            row.trusted,
            issuer_certificate_id,
            certificate_der,
        )
        .await?;
        return Ok(ImportedCertificate {
            id,
            created: false,
            idempotent: false,
        });
    }

    let id = insert_certificate(
        db,
        &metadata,
        metadata.is_ca,
        false,
        issuer_certificate_id,
        certificate_der,
        None,
    )
    .await?;
    Ok(ImportedCertificate {
        id,
        created: true,
        idempotent: false,
    })
}

struct CertificateMetadata {
    serial: Option<BigDecimal>,
    dn: Vec<u8>,
    country: Option<String>,
    state: Option<String>,
    locality: Option<String>,
    organization: Option<String>,
    organizational_unit: Option<String>,
    common_name: Option<String>,
    not_before: Option<OffsetDateTime>,
    not_after: Option<OffsetDateTime>,
    subject_key_id: Option<Vec<u8>>,
    authority_key_id: Option<Vec<u8>>,
    is_ca: bool,
}

impl CertificateMetadata {
    fn parse(
        certificate_der: &[u8],
    ) -> Result<Self, x509_parser::asn1_rs::Err<x509_parser::error::X509Error>> {
        let (_, parsed) = x509_parser::parse_x509_certificate(certificate_der)?;
        let mut metadata = Self::parse_subject(parsed.subject());
        metadata.serial = Some(BigDecimal::from_biguint(parsed.serial.clone(), 0));
        metadata.not_before = Some(parsed.validity().not_before.to_datetime());
        metadata.not_after = Some(parsed.validity().not_after.to_datetime());
        metadata.subject_key_id = subject_key_id(&parsed);
        metadata.authority_key_id = authority_key_id(&parsed);
        metadata.is_ca = parsed.is_ca();
        Ok(metadata)
    }

    fn parse_subject(dn: &X509Name<'_>) -> Self {
        Self {
            serial: None,
            dn: dn.as_raw().to_vec(),
            country: optional_rdn(dn.iter_country()),
            state: optional_rdn(dn.iter_state_or_province()),
            locality: optional_rdn(dn.iter_locality()),
            organization: optional_rdn(dn.iter_organization()),
            organizational_unit: optional_rdn(dn.iter_organizational_unit()),
            common_name: optional_rdn(dn.iter_common_name()),
            not_before: None,
            not_after: None,
            subject_key_id: None,
            authority_key_id: None,
            is_ca: false,
        }
    }
}

fn subject_key_id(certificate: &X509Certificate<'_>) -> Option<Vec<u8>> {
    certificate
        .iter_extensions()
        .find_map(|extension| match extension.parsed_extension() {
            ParsedExtension::SubjectKeyIdentifier(key_id) => Some(key_id.0.to_vec()),
            _ => None,
        })
}

fn authority_key_id(certificate: &X509Certificate<'_>) -> Option<Vec<u8>> {
    certificate
        .iter_extensions()
        .find_map(|extension| match extension.parsed_extension() {
            ParsedExtension::AuthorityKeyIdentifier(key_id) => key_id
                .key_identifier
                .as_ref()
                .map(|key_id| key_id.0.to_vec()),
            _ => None,
        })
}

fn optional_rdn<'n, I: Iterator<Item = &'n AttributeTypeAndValue<'n>>>(
    mut iter: I,
) -> Option<String> {
    iter.next().and_then(|x| x.as_str().ok()).map(String::from)
}

async fn insert_certificate<'e, E>(
    executor: E,
    metadata: &CertificateMetadata,
    is_ca: bool,
    trusted: bool,
    issuer_certificate_id: Option<i32>,
    der: &[u8],
    private_key_der: Option<&[u8]>,
) -> Result<i32, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query!(
        "insert into certificates (
            serial,
            dn,
            country,
            state,
            locality,
            organization,
            organizational_unit,
            common_name,
            not_before,
            not_after,
            der,
            private_key_der,
            issuer_certificate_id,
            is_ca,
            trusted,
            subject_key_id
        ) values (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            $9,
            $10,
            $11,
            $12,
            $13,
            $14,
            $15,
            $16
        )
        returning id",
        metadata.serial.as_ref(),
        &metadata.dn,
        metadata.country.as_deref(),
        metadata.state.as_deref(),
        metadata.locality.as_deref(),
        metadata.organization.as_deref(),
        metadata.organizational_unit.as_deref(),
        metadata.common_name.as_deref(),
        metadata.not_before,
        metadata.not_after,
        der,
        private_key_der,
        issuer_certificate_id,
        is_ca,
        trusted,
        metadata.subject_key_id.as_deref(),
    )
    .fetch_one(executor)
    .await?;
    Ok(row.id)
}

async fn insert_pending_certificate<'e, E>(
    executor: E,
    metadata: &CertificateMetadata,
    subject_key_id: &[u8],
    private_key_der: Option<&[u8]>,
) -> Result<i32, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query!(
        "insert into certificates (
            serial,
            dn,
            country,
            state,
            locality,
            organization,
            organizational_unit,
            common_name,
            not_before,
            not_after,
            der,
            private_key_der,
            issuer_certificate_id,
            is_ca,
            trusted,
            subject_key_id
        ) values (
            null,
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            null,
            null,
            null,
            $8,
            null,
            false,
            true,
            $9
        )
        returning id",
        &metadata.dn,
        metadata.country.as_deref(),
        metadata.state.as_deref(),
        metadata.locality.as_deref(),
        metadata.organization.as_deref(),
        metadata.organizational_unit.as_deref(),
        metadata.common_name.as_deref(),
        private_key_der,
        subject_key_id,
    )
    .fetch_one(executor)
    .await?;
    Ok(row.id)
}

async fn update_certificate(
    db: &PgPool,
    id: i32,
    metadata: &CertificateMetadata,
    is_ca: bool,
    trusted: bool,
    issuer_certificate_id: Option<i32>,
    der: &[u8],
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update certificates set
            serial = $1,
            dn = $2,
            country = $3,
            state = $4,
            locality = $5,
            organization = $6,
            organizational_unit = $7,
            common_name = $8,
            not_before = $9,
            not_after = $10,
            der = $11,
            issuer_certificate_id = $12,
            is_ca = $13,
            trusted = $14,
            subject_key_id = $15
        where id = $16",
        metadata.serial.as_ref(),
        &metadata.dn,
        metadata.country.as_deref(),
        metadata.state.as_deref(),
        metadata.locality.as_deref(),
        metadata.organization.as_deref(),
        metadata.organizational_unit.as_deref(),
        metadata.common_name.as_deref(),
        metadata.not_before,
        metadata.not_after,
        der,
        issuer_certificate_id,
        is_ca,
        trusted,
        metadata.subject_key_id.as_deref(),
        id,
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn find_certificate_by_subject_key_id(
    db: &PgPool,
    subject_key_id: &[u8],
) -> Result<Option<i32>, sqlx::Error> {
    let row = sqlx::query!(
        "select id from certificates where subject_key_id = $1",
        subject_key_id,
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|row| row.id))
}

fn verify_certificate_matches_private_key(
    certificate_der: &[u8],
    private_key_der: &[u8],
) -> Result<(), ImportCertificateError> {
    let (_, parsed) = x509_parser::parse_x509_certificate(certificate_der)?;
    let keypair = KeyPair::try_from(private_key_der.to_vec())?;
    if parsed.public_key().raw == keypair.subject_public_key_info().as_slice() {
        Ok(())
    } else {
        Err(ImportCertificateError::PublicKeyMismatch)
    }
}

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("no trusted certificate authority found")]
    MissingTrustedCa,
    #[error("issuer certificate #{0} not found")]
    MissingIssuer(i32),
    #[error(
        "issuer certificate #{0} is not a trusted certificate authority with stored key material"
    )]
    IssuerNotUsable(i32),
    #[error("active certificate authority is missing certificate der")]
    MissingStoredCertificateDer,
    #[error("active certificate authority is missing private key")]
    MissingStoredPrivateKey,
    #[error("malformed keypair, failed to parse {0}")]
    MalformedKeyPair(#[from] rcgen::Error),
    #[error("loading CA from database encounters an issue {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Error, Debug)]
pub enum InitializeError {
    #[error(transparent)]
    Generate(#[from] GenerateError),
    #[error("failed to parse generated CA certificate {0}")]
    InvalidCertificate(#[from] x509_parser::asn1_rs::Err<x509_parser::error::X509Error>),
    #[error("failed to persist CA {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Error, Debug)]
pub enum GenerateError {
    #[error("failed to generate or sign certificate authority {0}")]
    Rcgen(#[from] rcgen::Error),
}

#[derive(Error, Debug)]
pub enum IssueError {
    #[error("failed to generate certificate {0}")]
    Create(#[from] CreateError),
    #[error("failed to sign a certificate {0}")]
    Sign(#[from] rcgen::Error),
    #[error("specified date isn't valid or overflowed")]
    InvalidNotBefore,
}

#[derive(Error, Debug)]
pub enum TrackedIssueError {
    #[error(transparent)]
    Issue(#[from] IssueError),
    #[error("failed to track issued certificate: {0}")]
    Track(#[from] sqlx::Error),
    #[error("invalid certificate issued by issuer")]
    InvalidCertificate(#[from] x509_parser::asn1_rs::Err<x509_parser::error::X509Error>),
    #[error("issued certificate doesn't have a subject identifier")]
    MissingSubjectUid,
    #[error("issued certificate doesn't have a issuer identifier")]
    MissingIssuerUid,
}

#[derive(Error, Debug)]
pub enum PendingCsrError {
    #[error("specified date isn't valid or overflowed")]
    InvalidNotAfter,
    #[error("failed to create CSR {0}")]
    Create(#[from] CreateError),
    #[error("failed to serialize CSR {0}")]
    Serialize(#[from] rcgen::Error),
    #[error("generated CSR is invalid {0}")]
    InvalidCsr(x509_parser::asn1_rs::Err<x509_parser::error::X509Error>),
    #[error("failed to persist pending CSR {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Error, Debug)]
pub enum ImportCertificateError {
    #[error("invalid certificate {0}")]
    InvalidCertificate(#[from] x509_parser::asn1_rs::Err<x509_parser::error::X509Error>),
    #[error("imported certificate doesn't have a subject key identifier")]
    MissingSubjectKeyIdentifier,
    #[error("certificate #{0} already has different certificate DER")]
    CertificateAlreadyImported(i32),
    #[error("certificate public key doesn't match stored private key")]
    PublicKeyMismatch,
    #[error("stored private key is malformed {0}")]
    MalformedPrivateKey(#[from] rcgen::Error),
    #[error("failed to import certificate {0}")]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
mod tests {
    use rustls_pki_types::{CertificateDer, UnixTime};
    use time::{Duration, OffsetDateTime};
    use webpki::{
        ALL_VERIFICATION_ALGS, Cert, EndEntityCert, Error, KeyUsage, anchor_from_trusted_cert,
    };

    use crate::pki::ca::{CertificateAuthority, CertificateIssuer};

    #[tokio::test]
    async fn test_issue() {
        let ca = CertificateAuthority::generate().unwrap();
        let issued = ca
            .issue("testing subject", Duration::seconds(10))
            .await
            .unwrap();
        let ca_der = CertificateDer::from_slice(ca.certificate_der());
        let trust_anchor = anchor_from_trusted_cert(&ca_der).unwrap();
        let trust_anchors = [trust_anchor];

        let end_entity_der = CertificateDer::from_slice(issued.certificate.der());
        let end_entity = EndEntityCert::try_from(&end_entity_der).unwrap();
        let verified_path = end_entity
            .verify_for_usage(
                ALL_VERIFICATION_ALGS,
                &trust_anchors,
                &[],
                UnixTime::now(),
                KeyUsage::server_auth(),
                None,
                None,
            )
            .unwrap();
        assert!(verified_path.intermediate_certificates().next().is_none());

        let (_, ca_certificate) =
            x509_parser::parse_x509_certificate(ca.certificate_der()).unwrap();
        let (_, x509) = x509_parser::parse_x509_certificate(issued.certificate.der()).unwrap();
        assert!(ca_certificate.is_ca());
        assert!(!x509.is_ca());
        assert_eq!(&x509.issuer, ca_certificate.subject());
    }

    #[test]
    fn test_issue_expired() {
        let ca = CertificateAuthority::generate().unwrap();
        let issued = ca
            .issue_with_date(
                "testing subject",
                OffsetDateTime::now_utc().saturating_sub(Duration::hours(2)),
                OffsetDateTime::now_utc().saturating_sub(Duration::hours(1)),
            )
            .unwrap();
        let ca_der = CertificateDer::from_slice(ca.certificate_der());
        let trust_anchor = anchor_from_trusted_cert(&ca_der).unwrap();
        let trust_anchors = [trust_anchor];

        let end_entity_der = CertificateDer::from_slice(issued.certificate.der());
        let end_entity = EndEntityCert::try_from(&end_entity_der).unwrap();
        let verify_error = end_entity
            .verify_for_usage(
                ALL_VERIFICATION_ALGS,
                &trust_anchors,
                &[],
                UnixTime::now(),
                KeyUsage::server_auth(),
                None,
                None,
            )
            .map(|path| {
                path.intermediate_certificates()
                    .collect::<Vec<&Cert>>()
                    .len()
            })
            .expect_err("should be a invalid certificate");
        assert!(
            matches!(verify_error, Error::CertExpired { .. }),
            "{verify_error:?}"
        );
        let (_, ca_certificate) =
            x509_parser::parse_x509_certificate(ca.certificate_der()).unwrap();
        let (_, x509) = x509_parser::parse_x509_certificate(issued.certificate.der()).unwrap();
        assert!(!x509.is_ca());
        assert_eq!(&x509.issuer, ca_certificate.subject());
    }
}

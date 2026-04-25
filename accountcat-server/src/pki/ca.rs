use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};

use rcgen::{
    BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose,
    PKCS_ED25519,
};
use rustls_pki_types::CertificateDer;
use sqlx::{Executor, PgPool, Postgres, Row, types::BigDecimal};
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use tonic::async_trait;
use x509_parser::x509::AttributeTypeAndValue;

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
        let certificate_der: Option<Vec<u8>> = row.try_get("certificate_der")?;
        let certificate_der = certificate_der.ok_or(LoadError::MissingStoredCertificateDer)?;
        let keypair = KeyPair::try_from(row.try_get::<Vec<u8>, _>("private_key_der")?)?;
        Ok(Self {
            id: Some(row.try_get("id")?),
            keypair,
            certificate_der,
        })
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

    fn get_issuer(&self) -> Issuer<'static, &KeyPair> {
        let cert_der = CertificateDer::from_slice(self.certificate_der.as_slice());
        Issuer::from_ca_cert_der(&cert_der, &self.keypair)
            .expect("stored CA certificate should remain valid")
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

struct CertificateMetadata {
    serial: BigDecimal,
    dn: Vec<u8>,
    country: Option<String>,
    state: Option<String>,
    locality: Option<String>,
    organization: Option<String>,
    organizational_unit: Option<String>,
    common_name: Option<String>,
    not_before: OffsetDateTime,
    not_after: OffsetDateTime,
}

impl CertificateMetadata {
    fn parse(
        certificate_der: &[u8],
    ) -> Result<Self, x509_parser::asn1_rs::Err<x509_parser::error::X509Error>> {
        let (_, parsed) = x509_parser::parse_x509_certificate(certificate_der)?;
        let serial = BigDecimal::from_biguint(parsed.serial.clone(), 0);
        let dn = parsed.subject();
        Ok(Self {
            serial,
            dn: dn.as_raw().to_vec(),
            country: optional_rdn(dn.iter_country()),
            state: optional_rdn(dn.iter_state_or_province()),
            locality: optional_rdn(dn.iter_locality()),
            organization: optional_rdn(dn.iter_organization()),
            organizational_unit: optional_rdn(dn.iter_organizational_unit()),
            common_name: optional_rdn(dn.iter_common_name()),
            not_before: parsed.validity().not_before.to_datetime(),
            not_after: parsed.validity().not_after.to_datetime(),
        })
    }
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
    let row = sqlx::query(
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
            trusted
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
            $15
        )
        returning id",
    )
    .bind(&metadata.serial)
    .bind(&metadata.dn)
    .bind(&metadata.country)
    .bind(&metadata.state)
    .bind(&metadata.locality)
    .bind(&metadata.organization)
    .bind(&metadata.organizational_unit)
    .bind(&metadata.common_name)
    .bind(metadata.not_before)
    .bind(metadata.not_after)
    .bind(der)
    .bind(private_key_der)
    .bind(issuer_certificate_id)
    .bind(is_ca)
    .bind(trusted)
    .fetch_one(executor)
    .await?;
    row.try_get("id")
}

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("no trusted certificate authority found")]
    MissingTrustedCa,
    #[error("active certificate authority is missing certificate der")]
    MissingStoredCertificateDer,
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

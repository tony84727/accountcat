use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use rcgen::{Issuer, KeyPair, PKCS_ED25519};
use rustls_pki_types::CertificateDer;
use sqlx::{PgPool, types::BigDecimal};
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use tonic::async_trait;
use x509_parser::{nom::AsBytes, x509::AttributeTypeAndValue};

use crate::pki::{
    crt::IssuedCertificate,
    csr::{CreateError, ToBeSignedCertificate},
};

const KEYPAIR_SUBPATH: &str = "key.p8";
const CERTIFICATE_SUBPATH: &str = "ca.crt";

#[derive(Debug)]
pub struct CertificateAuthority {
    keypair: KeyPair,
    certificate_der: Vec<u8>,
}

impl PartialEq for CertificateAuthority {
    fn eq(&self, other: &Self) -> bool {
        self.keypair.algorithm() == other.keypair.algorithm()
            && self.keypair.serialized_der() == other.keypair.serialized_der()
    }
}

impl Eq for CertificateAuthority {}

impl CertificateAuthority {
    pub fn generate() -> Result<Self, GenerateError> {
        let keypair = KeyPair::generate_for(&PKCS_ED25519)?;
        let tbs = ToBeSignedCertificate::create(
            "Root CA",
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc().saturating_add(Duration::days(3650)),
        )?;
        let certificate = tbs.self_signed(&keypair)?;
        Ok(Self {
            keypair,
            certificate_der: certificate.der().to_vec(),
        })
    }

    pub fn save<P: AsRef<Path>>(&self, directory: P) -> Result<(), SaveError> {
        if !directory.as_ref().is_dir() {
            return Err(SaveError::NotDirectory);
        }
        let keypair_out = directory.as_ref().join(KEYPAIR_SUBPATH);
        let mut keypair = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(keypair_out)?;
        keypair.write_all(self.keypair.serialize_pem().as_bytes())?;
        let cert_out = directory.as_ref().join(CERTIFICATE_SUBPATH);
        let mut cert = File::create(cert_out)?;
        cert.write_all(&self.certificate_der)?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(directory: P) -> Result<Self, LoadError> {
        if !directory.as_ref().is_dir() {
            return Err(LoadError::NotDirectory);
        }
        let keypair_path = directory.as_ref().join(KEYPAIR_SUBPATH);
        let cert_path = directory.as_ref().join(CERTIFICATE_SUBPATH);
        if !keypair_path.is_file() {
            return Err(LoadError::MissingKeyPair(keypair_path));
        }
        if !cert_path.is_file() {
            return Err(LoadError::MissingCertificate(cert_path));
        }
        let mut cert_content = Vec::new();
        let mut cert = File::open(&cert_path)?;
        cert.read_to_end(&mut cert_content)?;
        let keypair = std::fs::read_to_string(keypair_path)?;
        let keypair = KeyPair::from_pem(&keypair)?;
        Ok(Self {
            keypair,
            certificate_der: cert_content.as_bytes().to_vec(),
        })
    }

    pub fn is_good<P: AsRef<Path>>(directory: P) -> bool {
        Self::load(directory).is_ok()
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
    async fn issue(
        &self,
        subject: &str,
        duration: Duration,
    ) -> Result<IssuedCertificate, IssueError> {
        Self::issue_with_date(
            &self,
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
        let (_, parsed) = x509_parser::parse_x509_certificate(issued.certificate.der())?;
        let serial = BigDecimal::from_biguint(parsed.serial.clone(), 0);
        let dn = parsed.subject();
        let raw_dn = parsed.subject().as_raw();
        fn optional_rdn<'n, I: Iterator<Item = &'n AttributeTypeAndValue<'n>>>(
            mut iter: I,
        ) -> Option<String> {
            iter.next().and_then(|x| x.as_str().ok()).map(String::from)
        }
        let country = optional_rdn(dn.iter_country());
        let state = optional_rdn(dn.iter_state_or_province());
        let locality = optional_rdn(dn.iter_locality());
        let organization = optional_rdn(dn.iter_organization());
        let organizational_unit = optional_rdn(dn.iter_organizational_unit());
        let common_name = optional_rdn(dn.iter_common_name());
        sqlx::query!(
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
                not_after
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
                $10
            )",
            serial,
            raw_dn,
            country,
            state,
            locality,
            organization,
            organizational_unit,
            common_name,
            issued.params.not_before,
            issued.params.not_after,
        )
        .execute(&self.db)
        .await?;
        Ok(issued)
    }
}

#[derive(Error, Debug)]
pub enum SaveError {
    #[error("saving target isn't a directory")]
    NotDirectory,
    #[error("saving CA encounters an IO issue")]
    IO(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("loading target isn't a directory")]
    NotDirectory,
    #[error("missing keypair or inaccessible at {0}")]
    MissingKeyPair(PathBuf),
    #[error("missing certificate or inaccessible at {0}")]
    MissingCertificate(PathBuf),
    #[error("malformed keypair, failed to parse {0}")]
    MalformedKeyPair(#[from] rcgen::Error),
    #[error("malformed certificate, failed to parse {0}")]
    MalformedCertificate(#[from] x509_parser::asn1_rs::Err<x509_parser::error::X509Error>),
    #[error("loading CA encounters an IO issue")]
    IO(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum GenerateError {
    #[error("failed create ")]
    SignCertificate(#[from] CreateError),
    #[error("failed to generate keypair: {0}")]
    GenerateKeyPair(#[from] rcgen::Error),
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
    use std::os::unix::fs::PermissionsExt;

    use rustls_pki_types::{CertificateDer, UnixTime};
    use temp_dir::TempDir;
    use time::{Duration, OffsetDateTime};
    use webpki::{
        ALL_VERIFICATION_ALGS, Cert, EndEntityCert, Error, KeyUsage, anchor_from_trusted_cert,
    };

    use crate::pki::ca::{CertificateAuthority, CertificateIssuer, KEYPAIR_SUBPATH};

    #[test]
    fn test_save_load() {
        let temp_dir = TempDir::new().expect("create temporary directory for testing");
        let ca = CertificateAuthority::generate().unwrap();
        ca.save(temp_dir.path()).unwrap();
        let loaded = CertificateAuthority::load(temp_dir.path()).unwrap();
        assert!(temp_dir.path().join(KEYPAIR_SUBPATH).is_file());
        assert_eq!(
            0o600,
            temp_dir
                .path()
                .join(KEYPAIR_SUBPATH)
                .metadata()
                .unwrap()
                .permissions()
                .mode()
                & 0o777
        );
        assert_eq!(ca, loaded);
    }

    #[tokio::test]
    async fn test_issue() {
        let ca = CertificateAuthority::generate().unwrap();
        let issued = ca
            .issue("testing subject", Duration::seconds(10))
            .await
            .unwrap();
        let ca_der = CertificateDer::from_slice(&ca.certificate_der);
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

        let (_, ca_certificate) = x509_parser::parse_x509_certificate(&ca.certificate_der).unwrap();
        let (_, x509) = x509_parser::parse_x509_certificate(issued.certificate.der()).unwrap();
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
        let ca_der = CertificateDer::from_slice(&ca.certificate_der);
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
        let (_, ca_certificate) = x509_parser::parse_x509_certificate(&ca.certificate_der).unwrap();
        let (_, x509) = x509_parser::parse_x509_certificate(issued.certificate.der()).unwrap();
        assert!(!x509.is_ca());
        assert_eq!(&x509.issuer, ca_certificate.subject());
    }
}

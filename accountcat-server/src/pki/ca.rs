use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use rcgen::{Certificate, Issuer, KeyPair, PKCS_ED25519};
use rustls_pki_types::CertificateDer;
use thiserror::Error;
use x509_parser::nom::AsBytes;

use crate::pki::csr::{CreateError, ToBeSignedCertificate};

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
        let tbs = ToBeSignedCertificate::create("Root CA")?;
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
        x509_parser::parse_x509_certificate(cert_content.as_bytes())?;
        let keypair = std::fs::read_to_string(keypair_path)?;
        let keypair = KeyPair::from_pem(&keypair)?;
        Ok(Self {
            keypair,
            certificate_der: cert_content.as_bytes().to_vec(),
        })
    }

    pub fn issue(&self, subject: &str) -> Result<Certificate, IssueError> {
        let tbs = ToBeSignedCertificate::create(subject)?;
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
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use rustls_pki_types::{CertificateDer, UnixTime};
    use temp_dir::TempDir;
    use webpki::{ALL_VERIFICATION_ALGS, EndEntityCert, KeyUsage, anchor_from_trusted_cert};

    use crate::pki::ca::{CertificateAuthority, KEYPAIR_SUBPATH};

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

    #[test]
    fn test_issue() {
        let ca = CertificateAuthority::generate().unwrap();
        let certificate = ca.issue("testing subject").unwrap();
        let ca_der = CertificateDer::from_slice(&ca.certificate_der);
        let trust_anchor = anchor_from_trusted_cert(&ca_der).unwrap();
        let trust_anchors = [trust_anchor];

        let end_entity_der = CertificateDer::from_slice(certificate.der());
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
        let (_, x509) = x509_parser::parse_x509_certificate(certificate.der()).unwrap();
        assert!(!x509.is_ca());
        assert_eq!(&x509.issuer, ca_certificate.subject());
    }
}

use rcgen::{Certificate, CertificateParams, KeyPair};

pub struct IssuedCertificate {
    pub key: KeyPair,
    pub params: CertificateParams,
    pub certificate: Certificate,
}

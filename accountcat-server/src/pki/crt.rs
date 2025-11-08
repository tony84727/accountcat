use rcgen::{Certificate, CertificateParams};

pub struct IssuedCertificate {
    pub params: CertificateParams,
    pub certificate: Certificate,
}

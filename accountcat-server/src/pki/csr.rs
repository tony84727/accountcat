use rcgen::{
    Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, Issuer, KeyPair,
    KeyUsagePurpose, PKCS_ED25519, SigningKey,
};
use thiserror::Error;

pub struct ToBeSignedCertificate {
    pub key: KeyPair,
    pub params: CertificateParams,
}

impl ToBeSignedCertificate {
    pub fn create(subject: &str) -> Result<Self, CreateError> {
        let mut params = CertificateParams::default();
        params.distinguished_name.push(DnType::CountryName, "TW");
        params
            .distinguished_name
            .push(DnType::OrganizationName, "Accountcat project");
        params.distinguished_name.push(DnType::CommonName, subject);
        params.key_usages.push(KeyUsagePurpose::DigitalSignature);
        params
            .extended_key_usages
            .push(ExtendedKeyUsagePurpose::ServerAuth);
        params
            .extended_key_usages
            .push(ExtendedKeyUsagePurpose::ClientAuth);
        let key = KeyPair::generate_for(&PKCS_ED25519).map_err(CreateError::Keypair)?;
        Ok(Self { key, params })
    }

    pub fn signed_by<S: SigningKey>(&self, issuer: &Issuer<S>) -> Certificate {
        self.params.signed_by(&self.key, issuer).unwrap()
    }
}

#[derive(Error, Debug)]
pub enum CreateError {
    #[error("unable to generate keypair {0}")]
    Keypair(rcgen::Error),
    #[error("unable to serialize certificate sign request {0}")]
    Serialize(rcgen::Error),
}

#[cfg(test)]
mod tests {
    use rcgen::{CertificateParams, Issuer, KeyPair};
    use x509_parser::nom::AsBytes;

    use crate::pki::csr::ToBeSignedCertificate;

    #[test]
    fn test_csr() {
        let tbs = ToBeSignedCertificate::create("testing").unwrap();
        let ca_keypair = KeyPair::generate().expect("generate ca keypair");
        let ca_params = CertificateParams::new(vec![String::from("testing-ca")]).unwrap();
        let ca_issuer = Issuer::from_params(&ca_params, &ca_keypair);
        let cert = tbs.signed_by(&ca_issuer);
        let (_, parsed) = x509_parser::parse_x509_certificate(cert.der().as_bytes()).unwrap();
        let rdn: Vec<String> = parsed
            .subject()
            .iter_rdn()
            .flat_map(|rdn| {
                rdn.iter().map(|a| {
                    format!(
                        "{}={}",
                        {
                            let oid = a.attr_type();
                            String::from(match oid.to_id_string().as_str() {
                                "2.5.4.3" => "CN",
                                "2.5.4.6" => "C",
                                "2.5.4.10" => "OU",
                                x => x,
                            })
                        },
                        a.as_str().unwrap()
                    )
                })
            })
            .collect();
        assert_eq!(vec!["CN=testing", "C=TW", "OU=Accountcat project"], rdn);
    }
}

use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use rcgen::{KeyPair, PKCS_ED25519};
use thiserror::Error;
const KEYPAIR_SUBPATH: &str = "key.p8";

#[derive(Debug)]
pub struct CertificateAuthority {
    keypair: KeyPair,
}

impl PartialEq for CertificateAuthority {
    fn eq(&self, other: &Self) -> bool {
        self.keypair.algorithm() == other.keypair.algorithm()
            && self.keypair.serialized_der() == other.keypair.serialized_der()
    }
}

impl Eq for CertificateAuthority {}

impl CertificateAuthority {
    pub fn generate() -> Result<Self, rcgen::Error> {
        Ok(Self {
            keypair: KeyPair::generate_for(&PKCS_ED25519)?,
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
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(directory: P) -> Result<Self, LoadError> {
        if !directory.as_ref().is_dir() {
            return Err(LoadError::NotDirectory);
        }
        let keypair_out = directory.as_ref().join(KEYPAIR_SUBPATH);
        if !keypair_out.is_file() {
            return Err(LoadError::MissingKeyPair(keypair_out));
        }
        let keypair = std::fs::read_to_string(keypair_out)?;
        let keypair = KeyPair::from_pem(&keypair)?;
        Ok(Self { keypair })
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
    #[error("malformed keypair, failed to parse {0}")]
    MalformedKeyPair(#[from] rcgen::Error),
    #[error("loading CA encounters an IO issue")]
    IO(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use temp_dir::TempDir;

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
}

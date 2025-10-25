use std::{
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::Args;
use openssl::{pkcs12::Pkcs12, pkey::PKey, x509::X509};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType, DnsName,
    ExtendedKeyUsagePurpose, IsCa, KeyUsagePurpose, PKCS_RSA_SHA256, SanType,
};

/// Arguments for generating PKI assets used by mutual TLS.
#[derive(Args, Debug, Clone)]
pub struct PkiArgs {
    /// Directory where generated artifacts are written.
    #[arg(long = "out-dir", default_value = "pki")]
    pub out_dir: PathBuf,

    /// Common name for the generated certificate authority (CA).
    #[arg(long = "ca-common-name", default_value = "Accountcat Local CA")]
    pub ca_common_name: String,

    /// Common name for the server certificate.
    #[arg(long = "server-common-name", default_value = "accountcat-server")]
    pub server_common_name: String,

    /// Subject alternative names (SANs) applied to the server certificate.
    #[arg(
        long = "server-san",
        value_name = "SAN",
        num_args = 1..,
        default_values_t = [String::from("localhost")]
    )]
    pub server_sans: Vec<String>,

    /// Common name for the client certificate.
    #[arg(long = "client-common-name", default_value = "accountcat-client")]
    pub client_common_name: String,

    /// Subject alternative names (SANs) applied to the client certificate.
    #[arg(long = "client-san", value_name = "SAN")]
    pub client_sans: Vec<String>,

    /// Optional password applied to the generated PKCS#12 bundles.
    #[arg(long = "pkcs12-password")]
    pub pkcs12_password: Option<String>,
}

impl Default for PkiArgs {
    fn default() -> Self {
        Self {
            out_dir: PathBuf::from("pki"),
            ca_common_name: "Accountcat Local CA".to_string(),
            server_common_name: "accountcat-server".to_string(),
            server_sans: vec!["localhost".to_string()],
            client_common_name: "accountcat-client".to_string(),
            client_sans: Vec::new(),
            pkcs12_password: None,
        }
    }
}

/// Generate certificate artifacts for mutual TLS.
pub fn generate(args: &PkiArgs) -> Result<()> {
    let out_dir = absolutize(&args.out_dir)?;
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create output directory '{}'", out_dir.display()))?;

    let ca = build_ca(&args.ca_common_name)?;
    let ca_der = ca.serialize_der()?;
    let ca_pem = ca.serialize_pem()?;
    let ca_key_pem = ca.serialize_private_key_pem();

    write_file(&out_dir, "ca.pem", ca_pem.as_bytes())?;
    write_file(&out_dir, "ca-key.pem", ca_key_pem.as_bytes())?;

    let server = build_server(&args.server_common_name, &args.server_sans)?;
    let server_der = server.serialize_der_with_signer(&ca)?;
    let server_pem = server.serialize_pem_with_signer(&ca)?;
    let server_key_pem = server.serialize_private_key_pem();
    let server_pkcs12 = build_pkcs12(
        &server,
        &server_der,
        &ca_der,
        args.pkcs12_password.as_deref(),
        &args.server_common_name,
    )?;

    write_file(&out_dir, "server.pem", server_pem.as_bytes())?;
    write_file(&out_dir, "server-key.pem", server_key_pem.as_bytes())?;
    write_file(&out_dir, "server.p12", &server_pkcs12)?;

    let client = build_client(&args.client_common_name, &args.client_sans)?;
    let client_der = client.serialize_der_with_signer(&ca)?;
    let client_pem = client.serialize_pem_with_signer(&ca)?;
    let client_key_pem = client.serialize_private_key_pem();
    let client_pkcs12 = build_pkcs12(
        &client,
        &client_der,
        &ca_der,
        args.pkcs12_password.as_deref(),
        &args.client_common_name,
    )?;

    write_file(&out_dir, "client.pem", client_pem.as_bytes())?;
    write_file(&out_dir, "client-key.pem", client_key_pem.as_bytes())?;
    write_file(&out_dir, "client.p12", &client_pkcs12)?;

    println!("Generated mTLS artifacts in {}", out_dir.display());

    Ok(())
}

fn build_ca(common_name: &str) -> Result<Certificate> {
    let mut params = CertificateParams::default();
    params.alg = &PKCS_RSA_SHA256;
    params.distinguished_name = distinguished_name(common_name);
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    params.serial_number = Some(rand::random());
    Certificate::from_params(params).context("failed to build CA certificate")
}

fn build_server(common_name: &str, sans: &[String]) -> Result<Certificate> {
    build_leaf_certificate(common_name, sans, ExtendedKeyUsagePurpose::ServerAuth)
}

fn build_client(common_name: &str, sans: &[String]) -> Result<Certificate> {
    build_leaf_certificate(common_name, sans, ExtendedKeyUsagePurpose::ClientAuth)
}

fn build_leaf_certificate(
    common_name: &str,
    sans: &[String],
    eku: ExtendedKeyUsagePurpose,
) -> Result<Certificate> {
    let mut params = CertificateParams::default();
    params.alg = &PKCS_RSA_SHA256;
    params.distinguished_name = distinguished_name(common_name);
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![eku];
    params.serial_number = Some(rand::random());
    params.subject_alt_names = parse_sans(sans)?;
    if eku == ExtendedKeyUsagePurpose::ServerAuth && params.subject_alt_names.is_empty() {
        bail!("server certificates require at least one subject alternative name");
    }
    Certificate::from_params(params)
        .with_context(|| format!("failed to build certificate for common name '{common_name}'"))
}

fn parse_sans(values: &[String]) -> Result<Vec<SanType>> {
    values
        .iter()
        .map(|value| {
            if let Ok(ip) = value.parse::<IpAddr>() {
                Ok(SanType::IpAddress(ip))
            } else {
                let dns = DnsName::try_from(value.clone())
                    .with_context(|| format!("invalid DNS name in SAN: {value}"))?;
                Ok(SanType::DnsName(dns))
            }
        })
        .collect()
}

fn build_pkcs12(
    certificate: &Certificate,
    certificate_der: &[u8],
    ca_der: &[u8],
    password: Option<&str>,
    friendly_name: &str,
) -> Result<Vec<u8>> {
    let key_der = certificate.serialize_private_key_der();
    let pkey = PKey::private_key_from_der(&key_der)
        .context("failed to parse private key for PKCS#12 export")?;
    let x509 = X509::from_der(certificate_der)
        .context("failed to parse signed certificate for PKCS#12 export")?;
    let ca_x509 =
        X509::from_der(ca_der).context("failed to parse CA certificate for PKCS#12 export")?;
    let mut builder = Pkcs12::builder();
    builder.ca(vec![ca_x509]);
    let pkcs12 = builder
        .build(password.unwrap_or(""), friendly_name, &pkey, &x509)
        .context("failed to construct PKCS#12 archive")?;
    pkcs12
        .to_der()
        .context("failed to encode PKCS#12 archive to DER format")
}

fn write_file(out_dir: &Path, name: &str, contents: &[u8]) -> Result<()> {
    let path = out_dir.join(name);
    fs::write(&path, contents)
        .with_context(|| format!("failed to write artifact '{}'", path.display()))
}

fn distinguished_name(common_name: &str) -> DistinguishedName {
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, common_name);
    dn
}

fn absolutize(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let cwd = std::env::current_dir().context("failed to determine current directory")?;
        Ok(cwd.join(path))
    }
}

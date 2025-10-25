use std::fs;

use accountcat::pki::{self, PkiArgs};
use anyhow::Result;
use openssl::{nid::Nid, pkcs12::Pkcs12};
use temp_dir::TempDir;

fn read_common_name(cert: &openssl::x509::X509Ref) -> String {
    cert.subject_name()
        .entries_by_nid(Nid::COMMONNAME)
        .next()
        .and_then(|entry| entry.data().as_utf8().ok())
        .map(|data| data.to_string())
        .expect("certificate missing common name")
}

#[test]
fn generates_pkcs12_bundles_with_expected_metadata() -> Result<()> {
    let temp = TempDir::new()?;
    let output = temp.path().join("artifacts");
    let args = PkiArgs {
        out_dir: output.clone(),
        ca_common_name: "Accountcat Test CA".to_string(),
        server_common_name: "Accountcat Test Server".to_string(),
        server_sans: vec!["example.local".to_string()],
        client_common_name: "Accountcat Test Client".to_string(),
        client_sans: vec!["client.example.local".to_string()],
        pkcs12_password: Some("secret".to_string()),
    };

    pki::generate(&args)?;

    // Server PKCS#12 bundle contains expected CN, SAN, and CA chain.
    let server_der = fs::read(output.join("server.p12"))?;
    let server = Pkcs12::from_der(&server_der)?.parse("secret")?;
    assert_eq!("Accountcat Test Server", read_common_name(&server.cert));
    let server_sans = server
        .cert
        .subject_alt_names()
        .expect("server certificate missing SAN");
    let dns_names: Vec<_> = server_sans
        .iter()
        .filter_map(|name| name.dnsname())
        .collect();
    assert!(dns_names.contains(&"example.local"));
    let ca_chain = server.chain.expect("server certificate missing CA chain");
    assert_eq!("Accountcat Test CA", read_common_name(&ca_chain[0]));

    // Client PKCS#12 bundle contains expected CN and SAN.
    let client_der = fs::read(output.join("client.p12"))?;
    let client = Pkcs12::from_der(&client_der)?.parse("secret")?;
    assert_eq!("Accountcat Test Client", read_common_name(&client.cert));
    let client_sans = client
        .cert
        .subject_alt_names()
        .expect("client certificate missing SAN");
    let client_dns: Vec<_> = client_sans
        .iter()
        .filter_map(|name| name.dnsname())
        .collect();
    assert!(client_dns.contains(&"client.example.local"));

    // CA artifacts exist for trust store distribution.
    assert!(output.join("ca.pem").exists());
    assert!(output.join("ca-key.pem").exists());

    Ok(())
}

use std::{fs::File, io::Write, process::exit};

use clap::{Parser, Subcommand};
use sqlx::PgPool;
use time::Duration;
use x509_parser::nom::AsBytes;

use crate::{
    config,
    pki::ca::{
        CertificateAuthority, CertificateIssuer, TrackedCertificateIssuer,
        create_option_for_sensitive_data,
    },
};

#[derive(Parser)]
pub struct Command {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// List issued certificates order by NotAfter in decreasing order
    List,
    /// Initialize Certificate Authority
    Init,
    /// Issue a certificate for entity
    Issue(IssueArgs),
}

async fn init() {
    let config = config::load().unwrap();
    if CertificateAuthority::is_good("./pki") {
        println!("A CA already initialized under ./pki");
        return;
    }
    let ca = CertificateAuthority::generate().unwrap();
    let ca_dir = config.pki.ca;
    std::fs::create_dir_all(&ca_dir).unwrap();
    ca.save(ca_dir).unwrap();
    println!("CA initialized successfully");
}

async fn list() {
    let config = config::load().unwrap();
    let pool: PgPool = config.database.into();
    let certificates = sqlx::query!(
        "select
            serial,
            country,
            state,
            locality,
            organization,
            organizational_unit,
            common_name,
            not_before,
            not_after
        from certificates
        order by not_after desc"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    if certificates.is_empty() {
        println!("<No Certificate>");
        return;
    }
    for c in certificates.into_iter() {
        let (serial, _) = c.serial.into_bigint_and_scale();
        println!("{:X}", serial);
        println!(
            "\tDN: C={},ST={},L={},O={},OU={},CN={}",
            c.country.unwrap_or_default(),
            c.state.unwrap_or_default(),
            c.locality.unwrap_or_default(),
            c.organization.unwrap_or_default(),
            c.organizational_unit.unwrap_or_default(),
            c.common_name.unwrap_or_default(),
        );
        println!("\tNotBefore: {}", c.not_before);
        println!("\tNotAfter: {}", c.not_after);
    }
}

#[derive(Parser)]
struct IssueArgs {
    /// Entity name
    subject: String,
    /// Certificate validity duration in days
    #[arg(default_value_t = 90)]
    days: i64,
}

impl IssueArgs {
    async fn run(&self) {
        let config = config::load().unwrap();
        let ca_dir = &config.pki.ca;
        let ca = CertificateAuthority::load(ca_dir).unwrap();
        let ca = TrackedCertificateIssuer::new(config.database.clone().into(), ca);
        let certificates_dir = ca_dir.join("certificates");
        let _ = std::fs::create_dir_all(&certificates_dir);
        if !certificates_dir.is_dir() {
            println!(
                "{} isn't a directory or inaccessible.",
                certificates_dir.to_string_lossy()
            );
            exit(1);
        }
        let issued = ca
            .issue(&self.subject, Duration::days(self.days))
            .await
            .unwrap();
        let (_, parsed) = x509_parser::parse_x509_certificate(issued.certificate.der())
            .expect("failed to parse issued certificate");
        let dir = certificates_dir.join(format!("{:X}", parsed.serial));
        if let Err(err) = std::fs::create_dir_all(&dir) {
            println!("failed to create certificate directory: {err:?}");
            exit(1);
        }
        let key_path = dir.join("key.crt");
        if let Err(err) = create_option_for_sensitive_data()
            .open(key_path)
            .and_then(|mut f| f.write_all(&issued.key.serialize_der()))
        {
            println!("failed to store certificate key: {err:?}");
            exit(1);
        }
        if let Err(err) = File::create(dir.join("crt.crt"))
            .and_then(|mut f| f.write_all(issued.certificate.der().as_bytes()))
        {
            println!("failed to store certificate file: {err:?}");
            exit(1);
        }
        println!("{}", issued.certificate.pem())
    }
}

impl Command {
    pub async fn run(&self) {
        match &self.action {
            Action::Init => init().await,
            Action::List => list().await,
            Action::Issue(args) => args.run().await,
        }
    }
}

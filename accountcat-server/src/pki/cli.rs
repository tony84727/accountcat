use clap::{Parser, Subcommand};
use sqlx::PgPool;

use crate::{config, pki::ca::CertificateAuthority};

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
        println!("\tNotBefore: {}", c.not_before.to_string());
        println!("\tNotAfter: {}", c.not_after.to_string());
    }
}

impl Command {
    pub async fn run(&self) {
        match self.action {
            Action::Init => init().await,
            Action::List => list().await,
        }
    }
}

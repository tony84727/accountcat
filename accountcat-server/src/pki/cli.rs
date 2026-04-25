use std::{fs::File, io::Write, process::exit};

use clap::{Args, Parser, Subcommand};
use sqlx::{FromRow, PgPool, types::BigDecimal};
use time::{Duration, OffsetDateTime};
use x509_parser::nom::AsBytes;

use crate::{
    config::Config,
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
    List(ListArgs),
    /// Initialize Certificate Authority
    Init,
    /// Issue a certificate for entity
    Issue(IssueArgs),
}

async fn init(config: &Config) {
    let pool: PgPool = config.database.clone().into();
    match CertificateAuthority::initialize(&pool).await {
        Ok(_) => println!("CA initialized successfully"),
        Err(err) => panic!("{err}"),
    }
}

#[derive(Args)]
struct ListArgs {
    /// List only certificate authorities
    #[arg(long)]
    ca: bool,
}

#[derive(FromRow)]
struct ListedCertificate {
    id: i32,
    serial: BigDecimal,
    country: Option<String>,
    state: Option<String>,
    locality: Option<String>,
    organization: Option<String>,
    organizational_unit: Option<String>,
    common_name: Option<String>,
    not_before: OffsetDateTime,
    not_after: OffsetDateTime,
    is_ca: bool,
    trusted: bool,
    has_private_key: bool,
}

impl ListedCertificate {
    fn can_issue(&self, now: OffsetDateTime) -> bool {
        self.is_ca && self.trusted && self.has_private_key && self.not_after > now
    }

    fn lines(&self, now: OffsetDateTime) -> Vec<String> {
        let (serial, _) = self.serial.clone().into_bigint_and_scale();
        vec![
            format!("Id: {}", self.id),
            format!("{:X}", serial),
            format!(
                "\tDN: C={},ST={},L={},O={},OU={},CN={}",
                self.country.clone().unwrap_or_default(),
                self.state.clone().unwrap_or_default(),
                self.locality.clone().unwrap_or_default(),
                self.organization.clone().unwrap_or_default(),
                self.organizational_unit.clone().unwrap_or_default(),
                self.common_name.clone().unwrap_or_default(),
            ),
            format!("\tCA: {}", yes_no(self.is_ca)),
            format!("\tCanIssue: {}", yes_no(self.can_issue(now))),
            format!("\tNotBefore: {}", self.not_before),
            format!("\tNotAfter: {}", self.not_after),
        ]
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

async fn list(config: &Config, args: &ListArgs) {
    let pool: PgPool = config.database.clone().into();
    let certificates = sqlx::query_as::<_, ListedCertificate>(
        "select
            id,
            serial,
            country,
            state,
            locality,
            organization,
            organizational_unit,
            common_name,
            not_before,
            not_after,
            is_ca,
            trusted,
            private_key_der is not null as has_private_key
        from certificates
        where (not $1 or is_ca)
        order by not_after desc",
    )
    .bind(args.ca)
    .fetch_all(&pool)
    .await
    .unwrap();
    if certificates.is_empty() {
        println!("<No Certificate>");
        return;
    }
    let now = OffsetDateTime::now_utc();
    for certificate in certificates {
        for line in certificate.lines(now) {
            println!("{line}");
        }
    }
}

#[derive(Parser)]
struct IssueArgs {
    /// Issuer certificate primary key from `pki list`
    #[arg(long)]
    issuer: i32,
    /// Entity name
    subject: String,
    /// Certificate validity duration in days
    #[arg(default_value_t = 90)]
    days: i64,
}

impl IssueArgs {
    async fn run(&self, config: &Config) {
        let pool: PgPool = config.database.clone().into();
        let ca = CertificateAuthority::load_by_id(&pool, self.issuer)
            .await
            .unwrap();
        let ca = TrackedCertificateIssuer::new(config.database.clone().into(), ca);
        let ca_dir = &config.pki.ca;
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
    pub async fn run(&self, config: &Config) {
        match &self.action {
            Action::Init => init(config).await,
            Action::List(args) => list(config, args).await,
            Action::Issue(args) => args.run(config).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, ListedCertificate};
    use clap::Parser;
    use sqlx::types::BigDecimal;
    use time::{Duration, OffsetDateTime};

    fn listed_certificate(
        is_ca: bool,
        trusted: bool,
        has_private_key: bool,
        not_after: OffsetDateTime,
    ) -> ListedCertificate {
        ListedCertificate {
            id: 12,
            serial: BigDecimal::from(255_i32),
            country: Some(String::from("TW")),
            state: Some(String::from("Taipei")),
            locality: Some(String::from("Taipei")),
            organization: Some(String::from("Accountcat")),
            organizational_unit: Some(String::from("PKI")),
            common_name: Some(String::from("testing")),
            not_before: OffsetDateTime::UNIX_EPOCH,
            not_after,
            is_ca,
            trusted,
            has_private_key,
        }
    }

    #[test]
    fn listed_certificate_reports_active_ca_as_issuable() {
        let now = OffsetDateTime::now_utc();
        let certificate = listed_certificate(true, true, true, now + Duration::days(1));

        let lines = certificate.lines(now);

        assert!(lines.iter().any(|line| line == "Id: 12"));
        assert!(lines.iter().any(|line| line == "\tCA: yes"));
        assert!(lines.iter().any(|line| line == "\tCanIssue: yes"));
    }

    #[test]
    fn listed_certificate_requires_private_key_to_issue() {
        let now = OffsetDateTime::now_utc();
        let certificate = listed_certificate(true, true, false, now + Duration::days(1));

        assert!(!certificate.can_issue(now));
    }

    #[test]
    fn listed_certificate_requires_unexpired_validity_to_issue() {
        let now = OffsetDateTime::now_utc();
        let certificate = listed_certificate(true, true, true, now - Duration::seconds(1));

        assert!(!certificate.can_issue(now));
    }

    #[test]
    fn listed_certificate_requires_trust_to_issue() {
        let now = OffsetDateTime::now_utc();
        let certificate = listed_certificate(true, false, true, now + Duration::days(1));

        assert!(!certificate.can_issue(now));
    }

    #[test]
    fn listed_certificate_requires_ca_to_issue() {
        let now = OffsetDateTime::now_utc();
        let certificate = listed_certificate(false, true, true, now + Duration::days(1));

        let lines = certificate.lines(now);

        assert!(lines.iter().any(|line| line == "\tCA: no"));
        assert!(lines.iter().any(|line| line == "\tCanIssue: no"));
    }

    #[test]
    fn issue_requires_issuer_argument() {
        let parsed = Command::try_parse_from(["accountcat", "issue", "testing"]);

        assert!(parsed.is_err());
    }

    #[test]
    fn issue_accepts_explicit_issuer_argument() {
        let parsed = Command::try_parse_from(["accountcat", "issue", "--issuer", "12", "testing"]);

        assert!(parsed.is_ok());
    }
}

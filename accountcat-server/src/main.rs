use accountcat::{
    config,
    pki::{self, PkiArgs},
    server::{self, ServerArg},
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    subcommand: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// run accountcat server
    Server(ServerArg),
    /// run database migration
    Migrate,
    /// Dump current server settings
    Settings,
    /// Generate PKI artifacts for mutual TLS setups
    Pki(PkiArgs),
}

impl Default for Command {
    fn default() -> Self {
        Command::Server(Default::default())
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.subcommand.unwrap_or_default() {
        Command::Server(arg) => server::main(&arg).await,
        Command::Migrate => accountcat::migration::run().await,
        Command::Settings => config::print_settings(),
        Command::Pki(args) => {
            if let Err(error) = pki::generate(&args) {
                eprintln!("error: {error:?}");
                std::process::exit(1);
            }
        }
    }
}

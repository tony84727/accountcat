use std::path::PathBuf;

use accountcat::{
    config::Config,
    pki,
    server::{self, ServerArg},
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Args {
    /// Alternative config file to load [default: server.toml]
    #[arg(short, long)]
    config: Option<PathBuf>,
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
    /// Public key infrastructure management
    Pki(pki::cli::Command),
}

impl Default for Command {
    fn default() -> Self {
        Command::Server(Default::default())
    }
}

#[tokio::main]
async fn main() {
    let Args { subcommand, config } = Args::parse();
    let config = Config::load(config).unwrap();
    match subcommand.unwrap_or_default() {
        Command::Server(arg) => server::main(&arg, &config).await,
        Command::Migrate => accountcat::migration::run(&config).await,
        Command::Settings => config.print_settings(),
        Command::Pki(pki_cli) => pki_cli.run(&config).await,
    }
}

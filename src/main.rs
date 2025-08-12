use accountcat::{config, server};
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    subcommand: Option<Command>,
}

#[derive(Subcommand, Default)]
enum Command {
    /// run accountcat server
    #[default]
    Server,
    /// run database migration
    Migrate,
    /// Dump current server settings
    Settings,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.subcommand.unwrap_or_default() {
        Command::Server => server::main().await,
        Command::Migrate => accountcat::migration::run().await,
        Command::Settings => config::print_settings(),
    }
}

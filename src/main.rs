use accountcat::server;
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    subcommand: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// run accountcat server
    Server,
    /// run database migration
    Migrate,
}

impl Default for Command {
    fn default() -> Self {
        Command::Server
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.subcommand.unwrap_or_default() {
        Command::Server => server::main().await,
        Command::Migrate => accountcat::migration::run().await,
    }
}

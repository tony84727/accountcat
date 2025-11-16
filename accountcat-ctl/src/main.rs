use accountcat::idl::user::user_client::UserClient;
use clap::{Parser, Subcommand};
use tonic::transport::{Channel, Uri};

#[derive(Parser)]
struct Arg {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Status,
}

async fn print_status() {
    let channel = Channel::from_static("http://localhost:3000")
        .connect()
        .await
        .unwrap();
    let mut client = UserClient::with_origin(channel, Uri::from_static("/grpc"));
    let response = client.get_profile(()).await.unwrap().into_inner();
    println!("Name: {}", response.name);
}

#[tokio::main]
async fn main() {
    let arg = Arg::parse();
    match arg.command {
        Command::Status => print_status().await,
    }
}

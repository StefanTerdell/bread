use std::str;
mod client;
mod server;
mod tcp_message;
use clap::{Parser, Subcommand};
use client::ClientError;
use server::ServerError;
use std::path::PathBuf;
use tcp_message::TcpMessage;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Box<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Open {
        #[arg(short, long)]
        port: Option<u16>,
        path: Option<PathBuf>,
    },
    Serve {
        #[arg(short = 'S', long)]
        shutdown_after_last: bool,
        #[arg(short, long)]
        silent: bool,
        #[arg(short = 'P', long)]
        print_port: bool,
        #[arg(short, long)]
        port: Option<u16>,
    },
}

#[derive(Debug)]
enum Error {
    Client(ClientError),
    Server(ServerError),
}

impl From<ClientError> for Error {
    fn from(err: ClientError) -> Error {
        Error::Client(err)
    }
}

impl From<ServerError> for Error {
    fn from(err: ServerError) -> Error {
        Error::Server(err)
    }
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match *cli.command {
        Commands::Open { port, path: _ } => client::connect_or_spawn_server(port)?,
        Commands::Serve {
            port,
            shutdown_after_last,
            silent,
            print_port,
        } => server::serve(port, shutdown_after_last, silent, print_port)?,
    };

    Ok(())
}

use std::default::Default;

use clap::{Parser, Subcommand};

mod api;
mod builder;
mod client;
mod hir;
mod ir;
mod nums;
mod range;
mod serdes;
mod server;
mod types;
mod var;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Default)]
enum Command {
    #[default]
    /// Run zap in the current directory and emit server and client code.
    Run,

    /// Create a new zap project in the current directory. This will make a new
    /// directory named `zap`.
    New,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Run => {
            let source = std::fs::read("./zap/net.luau").expect("failed to read zap/net.luau");
            let items = match api::exec(&source) {
                Ok(items) => items,
                Err(e) => {
                    eprintln!("error: {e}");
                    return;
                }
            };

            let server = server::server(server::Server::default(), &items)
                .expect("failed to generate server code");
            let client = client::client(client::Client::default(), &items)
                .expect("failed to generate client code");

            std::fs::create_dir_all("./zap/out").expect("failed to create zap/out directory");
            std::fs::write("./zap/out/server.luau", server).expect("failed to write server.luau");
            std::fs::write("./zap/out/client.luau", client).expect("failed to write client.luau");

            println!("Generated server and client code in `zap/out/` directory.");
        }

        Command::New => {
            std::fs::create_dir_all("./zap").expect("failed to create zap directory");
            std::fs::write("./zap/zap.d.luau", include_str!("zap.d.luau"))
                .expect("failed to write zap.d.luau");
            std::fs::write("./zap/net.luau", include_str!("net.luau"))
                .expect("failed to write net.luau");

            println!("Created a new zap project in the `zap` directory.");
            println!("Configure the `zap.d.luau` file as a definition file in your editor.");
        }
    }
}

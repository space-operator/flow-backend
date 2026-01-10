//! Standalone Command Server
//!
//! HTTP server that exposes flow-backend commands via REST endpoints.
//! Designed to run as a subprocess managed by Tauri.
//!
//! Startup protocol:
//! 1. Binds to port 0 (OS assigns available port)
//! 2. Prints `READY:{"port":XXXX}` to stdout
//! 3. Tauri parses this line to discover the port

// Command crates - importing registers them via inventory
use cmds_deno as _;
use cmds_pdg as _;
use cmds_solana as _;
use cmds_std as _;
use rhai_script as _;

use actix_web::{web, App, HttpResponse, HttpServer};
use clap::Parser;
use flow_lib::command::{CommandDescription, MatchName};
use serde::Serialize;
use std::net::TcpListener;

#[derive(Parser, Debug)]
#[command(name = "command-server")]
#[command(about = "Standalone HTTP server for flow-backend commands")]
struct Args {
    /// Solana RPC URL
    #[arg(long, default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,
}

/// Command information returned from /commands endpoint
#[derive(Serialize, Clone, Debug)]
struct CommandInfo {
    name: String,
    command_type: String,
}

/// Server info returned from /info endpoint
#[derive(Serialize)]
struct ServerInfo {
    version: &'static str,
    commands: usize,
    rpc_url: String,
}

/// Ready message printed to stdout for Tauri to parse
#[derive(Serialize)]
struct ReadyMessage {
    port: u16,
}

fn get_commands() -> Vec<CommandInfo> {
    inventory::iter::<CommandDescription>()
        .map(|desc| {
            let name = match &desc.matcher.name {
                MatchName::Exact(cow) => cow.to_string(),
                MatchName::Regex(cow) => cow.to_string(),
            };
            CommandInfo {
                name,
                command_type: format!("{:?}", desc.matcher.r#type),
            }
        })
        .collect()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let rpc_url = args.rpc_url.clone();

    // Bind to port 0 to get an available port from the OS
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();

    // Print ready message for Tauri to parse
    let ready = ReadyMessage { port };
    println!("READY:{}", serde_json::to_string(&ready).unwrap());

    // Flush stdout to ensure Tauri receives the message immediately
    use std::io::Write;
    std::io::stdout().flush()?;

    let rpc_for_info = rpc_url.clone();

    HttpServer::new(move || {
        let rpc = rpc_for_info.clone();

        App::new()
            .route(
                "/health",
                web::get().to(|| async { HttpResponse::Ok().body("OK") }),
            )
            .route(
                "/commands",
                web::get().to(|| async { HttpResponse::Ok().json(get_commands()) }),
            )
            .route(
                "/info",
                web::get().to(move || {
                    let rpc = rpc.clone();
                    async move {
                        HttpResponse::Ok().json(ServerInfo {
                            version: env!("CARGO_PKG_VERSION"),
                            commands: inventory::iter::<CommandDescription>().count(),
                            rpc_url: rpc,
                        })
                    }
                }),
            )
    })
    .listen(listener)?
    .run()
    .await
}

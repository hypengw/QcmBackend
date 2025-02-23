use log::LevelFilter;
use std::{env, io::Error, path::PathBuf, str::FromStr};

use futures_util::{future, StreamExt, TryStreamExt};
use log::info;
use tokio::net::{TcpListener, TcpStream};

use anyhow;
use clap::{self, Parser};
use sea_orm::Database;

use migration::Migrator;
#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Data directory path
    #[arg(short, long)]
    data: PathBuf,

    /// Port to listen on, auto if not set
    #[arg(short, long)]
    port: Option<u16>,

    /// Log level (error, warn, info, debug, trace)
    #[arg(short, long, env = "RUST_LOG")]
    log_level: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(
            args.log_level
                .and_then(|l| LevelFilter::from_str(&l).ok())
                .unwrap_or(LevelFilter::Warn),
        )
        .init();

    // Use port 0 if none specified (system will assign an available port)
    let port = args.port.unwrap_or(0);
    let addr = format!("127.0.0.1:{}", port);

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");

    // Get the actual local address (including system-assigned port if port was 0)
    let local_addr = listener.local_addr().expect("Failed to get local address");
    info!("Listening on: {}", local_addr);

    prepare_db(args.data).await?;

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }

    Ok(())
}

async fn prepare_db(data: PathBuf) -> Result<(), anyhow::Error> {
    let db_path = data.join("backend.db");
    let db_url = format!(
        "sqlite://{}?mode=rwc&journal_mode=WAL",
        db_path.to_string_lossy()
    );
    let db = Database::connect(&db_url).await?;

    Migrator::up(&db, None).await?;

    Ok(())
}

async fn accept_connection(stream: TcpStream) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    info!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (write, read) = ws_stream.split();
    // We should not forward messages other than text or binary.
    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
        .forward(write)
        .await
        .expect("Failed to forward messages")
}

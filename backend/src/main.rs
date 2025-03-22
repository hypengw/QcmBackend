use anyhow;
use clap::{self, Parser};
use event::{BackendContext, BackendEvent};
use log::LevelFilter;
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf, str::FromStr};

use futures_util::{future, SinkExt, StreamExt, TryStreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};

use qcm_core::provider::Context;

mod api;
mod convert;
mod error;
mod event;
mod task;
mod global;
mod msg;

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

    qcm_core::global::init(&args.data);
    qcm_plugins::init();

    let db = prepare_db(args.data).await?;
    qcm_core::global::load_from_db(&db).await;

    let listener = {
        // Use port 0 if none specified (system will assign an available port)
        let port = args.port.unwrap_or(0);
        let addr = format!("127.0.0.1:{}", port);

        let try_socket = TcpListener::bind(&addr).await;
        let listener = try_socket.expect("Failed to bind");

        let local_addr = listener.local_addr().expect("Failed to get local address");

        // print port json
        println!(
            "{}",
            serde_json::to_string(&HashMap::from([("port", local_addr.port())])).unwrap()
        );
        listener
    };

    // only need the first accept connection
    if let Ok((stream, _)) = listener.accept().await {
        let db = db.clone();
        let handle = tokio::spawn(accept_connection(stream, db));
        handle.await?;
    }

    Ok(())
}

async fn prepare_db(data: PathBuf) -> Result<DatabaseConnection, anyhow::Error> {
    let db_path = data.join("backend.db");
    // TODO: add journal_mode=wal support
    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

    let db = Database::connect(&db_url).await?;

    Migrator::up(&db, None).await?;

    Ok(db)

    // let pool = SqlitePool::connect(&db_url).await?;
    // Ok(pool)
}

async fn accept_connection(stream: TcpStream, db: DatabaseConnection) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    log::info!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    log::info!("New connection: {}", addr);

    let (ws_sender, mut ws_receiver) = mpsc::channel::<WsMessage>(32);

    let (ev_sender, mut ev_receiver) = mpsc::channel::<event::Event>(32);
    let (bk_ev_sender, mut bk_ev_receiver) = mpsc::channel::<event::BackendEvent>(32);

    let (mut ws_writer, ws_reader) = ws_stream.split();

    let ctx = Arc::new(BackendContext {
        provider_context: Arc::new(Context {
            db,
            ev_sender: ev_sender,
        }),
        bk_ev_sender,
        ws_sender,
    });

    // ws sender queue
    tokio::spawn(async move {
        while let Some(msg) = ws_receiver.recv().await {
            if ws_writer.send(msg.into()).await.is_err() {
                break;
            }
        }
        log::info!("Channel recv end");
    });

    // event queue
    tokio::spawn({
        let ctx = ctx.clone();
        async move {
            while let Some(ev) = ev_receiver.recv().await {
                match api::process_event(ev, ctx.clone()).await {
                    Ok(true) => break,
                    Err(err) => log::error!("{}", err),
                    _ => (),
                }
            }
            log::info!("Event channel recv end");
        }
    });

    // backend event queue
    tokio::spawn({
        let ctx = ctx.clone();
        async move {
            while let Some(ev) = bk_ev_receiver.recv().await {
                match api::process_backend_event(ev, ctx.clone()).await {
                    Ok(true) => break,
                    Err(err) => log::error!("{}", err),
                    _ => (),
                }
            }
            log::info!("Backend event channel recv end");
        }
    });

    let _ = ctx.bk_ev_sender.try_send(BackendEvent::Frist);

    // receive from ws
    // let mut read = ws_reader.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()));
    let mut reader = ws_reader;
    while let Ok(Some(message)) = reader.next().await.transpose() {
        tokio::spawn({
            let ctx = ctx.clone();
            async move {
                if let Err(e) = api::handler::handle_message(message, ctx).await {
                    log::warn!("Error processing message: {}", e);
                }
            }
        });
    }

    // end event process
    ctx.provider_context
        .ev_sender
        .send(event::Event::End)
        .await
        .unwrap();
    ctx.bk_ev_sender
        .send(event::BackendEvent::End)
        .await
        .unwrap();
    log::info!("Connection end: {}", addr);
}

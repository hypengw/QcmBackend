use anyhow;
use clap::{self, Parser};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};
use task::TaskManager;

use hyper::{body::Incoming, Request};
use hyper_util::rt::TokioIo;
use tokio::{
    net::{TcpListener, TcpStream},
    signal::unix::{signal, SignalKind},
    sync::watch,
};

use migration::{CacheDBMigrator, Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

mod api;
mod convert;
mod error;
mod event;
mod global;
mod msg;
mod reverse;
mod task;

use api::handler::handle_request;

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
    use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, reload};
    let (filter, reload_handle) = reload::Layer::new(LevelFilter::ERROR);
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::Layer::default())
        .init();

    let args = Args::parse();
    let log_level = args
        .log_level
        .and_then(|l| LevelFilter::from_str(&l).ok())
        .unwrap_or(LevelFilter::ERROR);

    qcm_core::global::init(&args.data);
    qcm_plugins::init();

    let (oper, taskmgr_handle) = {
        let (oper, mgr) = TaskManager::new();
        (oper, mgr.start())
    };

    let db = prepare_db(&args.data).await?;
    let cache_db = prepare_cache_db(&args.data).await?;

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    global::set_shutdown_tx(shutdown_tx.clone());

    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        loop {
            tokio::select! {
                _ = sigterm.recv() => {}
                _ = sigint.recv() => {}
            };
            shutdown_tx.send(true).unwrap();
        }
    });

    // Use port 0 if none specified (system will assign an available port)
    let listener = listen(args.port.unwrap_or(0)).await;

    let _ = reload_handle.modify(|f| {
        *f = log_level;
    });

    qcm_core::global::load_from_db(&db).await;

    let accept = |stream: TcpStream| {
        let addr = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        log::info!("New connection: {}", addr);
        let db = db.clone();
        let cache_db = cache_db.clone();
        let oper = oper.clone();
        tokio::spawn(async move {
            let http = hyper::server::conn::http1::Builder::new();
            let service = |request: Request<Incoming>| {
                let db = db.clone();
                let cache_db = cache_db.clone();
                let oper = oper.clone();
                async move { handle_request(request, db, cache_db, oper).await }
            };
            let connection = http
                .serve_connection(TokioIo::new(stream), hyper::service::service_fn(service))
                .with_upgrades();

            if let Err(err) = connection.await {
                log::error!("Error HTTP connection: {err:?}");
            }
        });
    };

    loop {
        if *shutdown_rx.borrow() {
            break;
        }

        tokio::select! {
            accept_result = listener.accept() => {
                if let Ok((stream, _)) = accept_result {
                    accept(stream);
                }
            }
            _ = shutdown_rx.changed() => {
                log::info!("Shutting down...");
                break;
            }
        }
    }

    // wait stop
    {
        oper.stop();
        let _ = taskmgr_handle.join();
    }
    Ok(())
}

async fn prepare_db(data: &Path) -> Result<DatabaseConnection, anyhow::Error> {
    let db_path = data.join("backend.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

    let mut opt = sea_orm::ConnectOptions::new(db_url);
    opt.sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Debug)
        .sqlx_slow_statements_logging_settings(log::LevelFilter::Debug, Duration::from_secs(1));

    // mmap_size 128MB
    // journal_size_limit 64MB
    // cache_size 8MB
    let db = Database::connect(opt).await?;
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 134217728;
            PRAGMA journal_size_limit = 67108864;
            PRAGMA cache_size = 2000;
        "
        .to_owned(),
    ))
    .await?;

    // custom migrator
    Migrator::down(&db, None).await?;
    Migrator::up(&db, None).await?;

    Ok(db)
}

async fn prepare_cache_db(data: &Path) -> Result<DatabaseConnection, anyhow::Error> {
    let db_path = data.join("backend_cache.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

    let mut opt = sea_orm::ConnectOptions::new(db_url);
    opt.sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Debug)
        .sqlx_slow_statements_logging_settings(log::LevelFilter::Debug, Duration::from_secs(1));

    // mmap_size 128MB
    // journal_size_limit 64MB
    // cache_size 8MB
    let db = Database::connect(opt).await?;
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 134217728;
            PRAGMA journal_size_limit = 67108864;
            PRAGMA cache_size = 2000;
        "
        .to_owned(),
    ))
    .await?;

    // custom migrator
    CacheDBMigrator::down(&db, None).await?;
    CacheDBMigrator::up(&db, None).await?;

    Ok(db)
}

async fn listen(port: u16) -> TcpListener {
    let listener = {
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
    listener
}

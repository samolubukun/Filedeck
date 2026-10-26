use crate::config::Config;
use crate::ingest;
use crate::routes::{self, create_router};
use crate::search::{self, init_search_index};
use crate::state::AppState;

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

/// Starts the complete FileDeck core engine:
/// 1. Initializes storage directories
/// 2. Spawns/connects the Python worker (supporting both frozen standalone binaries & dev venv)
/// 3. Runs SQLite database migrations
/// 4. Initializes Tantivy search index
/// 5. Starts folder watcher & mail poller
/// 6. Binds Axum HTTP / WebSocket server
pub async fn start_server() -> anyhow::Result<()> {
    tracing::info!("========================================");
    tracing::info!("       Starting FileDeck Core           ");
    tracing::info!("========================================");

    let config = Config::load();

    // Ensure all data directories exist
    std::fs::create_dir_all(&config.storage.media_root)?;
    std::fs::create_dir_all(config.storage.media_root.join("originals"))?;
    std::fs::create_dir_all(config.storage.media_root.join("archive"))?;
    std::fs::create_dir_all(config.storage.media_root.join("thumbnails"))?;
    std::fs::create_dir_all(&config.storage.consume_dir)?;
    std::fs::create_dir_all(&config.storage.index_dir)?;
    std::fs::create_dir_all(&config.storage.share_link_bundle_dir)?;

    // Spawn Python worker subprocess if not already running
    let worker_url = config.worker.url.clone();
    let worker_client = reqwest::Client::builder().build()?;
    let is_worker_up = worker_client
        .get(format!("{}/health", worker_url))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    let mut _worker_child = if !is_worker_up {
        tracing::info!("Checking for FileDeck worker (standalone or Python)...");

        // 1. Check for frozen standalone binary candidates (Tauri bundle resources)
        let frozen_candidates = vec![
            PathBuf::from("resources/worker/filedeck-worker.exe"),
            PathBuf::from("resources/worker/filedeck-worker"),
            PathBuf::from("../src-tauri/resources/worker/filedeck-worker.exe"),
            PathBuf::from("../src-tauri/resources/worker/filedeck-worker"),
            PathBuf::from("filedeck-worker.exe"),
            PathBuf::from("filedeck-worker"),
        ];

        let mut child = None;
        for candidate in frozen_candidates {
            if candidate.exists() {
                tracing::info!("Found frozen standalone worker: {}", candidate.display());
                if let Ok(c) = std::process::Command::new(&candidate).spawn() {
                    child = Some(c);
                    break;
                }
            }
        }

        // 2. If no frozen binary, fallback to Python dev environment candidates
        if child.is_none() {
            let python_candidates = vec![
                PathBuf::from("../worker/.venv/Scripts/python.exe"),
                PathBuf::from("worker/.venv/Scripts/python.exe"),
                PathBuf::from("../worker/.venv/bin/python"),
                PathBuf::from("worker/.venv/bin/python"),
                PathBuf::from("python"),
                PathBuf::from("python3"),
            ];

            for candidate in python_candidates {
                if candidate.exists() || candidate == PathBuf::from("python") || candidate == PathBuf::from("python3") {
                    let current_dir = if PathBuf::from("../worker").exists() {
                        PathBuf::from("../worker")
                    } else {
                        PathBuf::from("worker")
                    };

                    let attempt = std::process::Command::new(&candidate)
                        .args([
                            "-m",
                            "uvicorn",
                            "app.main:app",
                            "--host",
                            "127.0.0.1",
                            "--port",
                            "8900",
                        ])
                        .current_dir(current_dir)
                        .spawn();

                    if let Ok(c) = attempt {
                        child = Some(c);
                        break;
                    }
                }
            }
        }

        // Wait up to 5 seconds for worker health
        for _ in 0..25 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            if worker_client
                .get(format!("{}/health", worker_url))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
            {
                tracing::info!("FastAPI worker health check passed.");
                break;
            }
        }
        child
    } else {
        tracing::info!("Existing FastAPI worker detected at {}", worker_url);
        None
    };

    // Connect SQLite database
    let db_opts = sqlx::sqlite::SqliteConnectOptions::from_str(&config.database.url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

    let db_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(db_opts)
        .await?;

    tracing::info!("Applying SQLite database migrations...");
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    tracing::info!("Initializing Tantivy full-text index...");
    let bundle = init_search_index(&config.storage.index_dir)?;

    let (ws_tx, _) = broadcast::channel(100);

    let app_state = AppState {
        db: db_pool.clone(),
        config: Arc::new(config.clone()),
        ws_tx: ws_tx.clone(),
        worker_client,
        index: bundle.index,
        index_writer: bundle.writer,
        index_reader: bundle.reader,
        search_schema: bundle.schema,
        search_fields: bundle.fields,
    };

    let db_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE deleted_at IS NULL")
            .fetch_one(&app_state.db)
            .await?;
    if bundle.needs_rebuild || search::indexed_doc_count(&app_state) != db_count as u64 {
        let n = search::reindex_all(&app_state).await?;
        tracing::info!("Search index rebuilt with {} documents", n);
    }

    ingest::watcher::spawn(app_state.clone());
    routes::mail::spawn_mail_poller(app_state.clone());

    // Documents stay in the trash for 30 days, then are purged.
    {
        let st = app_state.clone();
        tokio::spawn(async move {
            loop {
                let cutoff = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();
                let expired: Vec<i64> = sqlx::query_scalar(
                    "SELECT id FROM documents WHERE deleted_at IS NOT NULL AND deleted_at < $1",
                )
                .bind(cutoff)
                .fetch_all(&st.db)
                .await
                .unwrap_or_default();
                if !expired.is_empty() {
                    tracing::info!("Purging {} documents from the trash", expired.len());
                    if let Err(e) = routes::documents_ops::purge_documents(&st, &expired).await {
                        tracing::warn!("Trash purge failed: {e}");
                    }
                }
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        });
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let mut router = create_router(app_state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    // Serve Frontend SPA if built
    let frontend_dist_candidates = vec![
        PathBuf::from("../frontend/dist"),
        PathBuf::from("frontend/dist"),
    ];

    for dist in frontend_dist_candidates {
        if dist.exists() {
            let index_html = dist.join("index.html");
            if index_html.exists() {
                tracing::info!("Serving FileDeck frontend UI from {}", dist.display());
                let serve_dir = ServeDir::new(&dist).fallback(ServeFile::new(&index_html));
                router = router.fallback_service(serve_dir);
                break;
            }
        }
    }

    let addr = format!("{}:{}", config.app.host, config.app.port);
    tracing::info!(
        "FileDeck Core ready! Open your browser at: http://localhost:{}",
        config.app.port
    );

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

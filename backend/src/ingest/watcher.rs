//! Consume-folder watcher.
//!
//! Files appearing in `consume_dir` are waited on until their size is stable, ingested, and
//! removed on success. Files that fail (or are duplicates) stay in place and are not retried
//! until they change on disk.

use super::{consume_and_wait, is_supported, ConsumeRequest};
use crate::state::AppState;
use notify::{EventKind, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;

const SETTLE: Duration = Duration::from_secs(2);
const TICK: Duration = Duration::from_secs(1);
const RESCAN: Duration = Duration::from_secs(30);

fn ignored(state: &AppState, path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    state
        .config
        .consumer
        .ignore_patterns
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .any(|p| {
            p.matches_with(
                name,
                glob::MatchOptions {
                    case_sensitive: false,
                    require_literal_separator: false,
                    require_literal_leading_dot: false,
                },
            )
        })
}

fn candidate(state: &AppState, path: &Path) -> bool {
    path.is_file()
        && !ignored(state, path)
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .map(is_supported)
            .unwrap_or(false)
}

fn fingerprint(path: &Path) -> Option<(u64, SystemTime)> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.len(), m.modified().ok()?))
}

fn scan(state: &AppState, root: &Path, recursive: bool, out: &mut Vec<PathBuf>) {
    let depth = if recursive { usize::MAX } else { 1 };
    for entry in walkdir::WalkDir::new(root)
        .max_depth(depth)
        .into_iter()
        .flatten()
    {
        if candidate(state, entry.path()) {
            out.push(entry.path().to_path_buf());
        }
    }
}

/// Start watching `consume_dir` in the background.
pub fn spawn(state: AppState) {
    if !state.config.consumer.watch {
        tracing::info!("Consume folder watching is disabled");
        return;
    }
    tokio::spawn(async move {
        if let Err(e) = run(state).await {
            tracing::error!("Consume folder watcher stopped: {e}");
        }
    });
}

async fn run(state: AppState) -> anyhow::Result<()> {
    let root = state.config.storage.consume_dir.clone();
    std::fs::create_dir_all(&root)?;
    let recursive = state.config.consumer.recursive;

    let (tx, mut rx) = mpsc::unbounded_channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(
        &root,
        if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        },
    )?;
    tracing::info!("Watching consume folder {}", root.display());

    // path -> (last event time, last seen size/mtime)
    let mut pending: HashMap<PathBuf, (Instant, Option<(u64, SystemTime)>)> = HashMap::new();
    let mut processing: HashSet<PathBuf> = HashSet::new();
    let mut failed: HashMap<PathBuf, (u64, SystemTime)> = HashMap::new();
    let (done_tx, mut done_rx) = mpsc::unbounded_channel::<(PathBuf, bool)>();

    let mut initial = Vec::new();
    scan(&state, &root, recursive, &mut initial);
    for p in initial {
        pending.insert(p, (Instant::now() - SETTLE, None));
    }

    let mut tick = tokio::time::interval(TICK);
    let mut last_rescan = Instant::now();

    loop {
        tokio::select! {
            Some(res) = rx.recv() => {
                if let Ok(ev) = res {
                    if matches!(ev.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                        for p in ev.paths {
                            if candidate(&state, &p) {
                                pending.insert(p, (Instant::now(), None));
                            }
                        }
                    }
                }
            }
            Some((path, ok)) = done_rx.recv() => {
                processing.remove(&path);
                if !ok {
                    if let Some(fp) = fingerprint(&path) {
                        failed.insert(path, fp);
                    }
                }
            }
            _ = tick.tick() => {
                if last_rescan.elapsed() >= RESCAN {
                    last_rescan = Instant::now();
                    let mut found = Vec::new();
                    scan(&state, &root, recursive, &mut found);
                    for p in found {
                        if !processing.contains(&p) && !pending.contains_key(&p) {
                            pending.insert(p, (Instant::now() - SETTLE, None));
                        }
                    }
                }

                let ready: Vec<PathBuf> = pending
                    .iter()
                    .filter(|(_, (t, _))| t.elapsed() >= SETTLE)
                    .map(|(p, _)| p.clone())
                    .collect();

                for path in ready {
                    let Some(now_fp) = fingerprint(&path) else {
                        pending.remove(&path);
                        continue;
                    };
                    // Wait until the file stopped growing.
                    let stable = matches!(pending.get(&path), Some((_, Some(prev))) if *prev == now_fp);
                    if !stable {
                        pending.insert(path, (Instant::now() - SETTLE + Duration::from_secs(1), Some(now_fp)));
                        continue;
                    }
                    pending.remove(&path);
                    if processing.contains(&path) || failed.get(&path) == Some(&now_fp) {
                        continue;
                    }
                    failed.remove(&path);
                    processing.insert(path.clone());

                    let st = state.clone();
                    let tx = done_tx.clone();
                    tokio::spawn(async move {
                        let ok = consume_file(&st, &path).await;
                        let _ = tx.send((path, ok));
                    });
                }
            }
        }
    }
}

async fn consume_file(state: &AppState, path: &Path) -> bool {
    let mut bytes = match tokio::fs::read(path).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Cannot read {}: {e}", path.display());
            return false;
        }
    };
    let mut filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document")
        .to_string();

    // Double-sided ADF scan check
    let is_double_sided = state.config.consumer.enable_collate_double_sided
        || path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy().to_ascii_lowercase();
            s == "double-sided" || s == "doublesided" || s == "double_sided"
        });

    if is_double_sided {
        let scratch_dir = state.config.storage.media_root.join("scratch");
        let _ = tokio::fs::create_dir_all(&scratch_dir).await;
        let staging = scratch_dir.join("double-sided-staging.pdf");

        let mut valid_staging = false;
        if staging.exists() {
            if let Ok(meta) = tokio::fs::metadata(&staging).await {
                if let Ok(modified) = meta.modified() {
                    if let Ok(dur) = SystemTime::now().duration_since(modified) {
                        if dur <= Duration::from_secs(1800) {
                            valid_staging = true;
                        } else {
                            tracing::warn!("Outdated double-sided staging file exists, removing it");
                            let _ = tokio::fs::remove_file(&staging).await;
                        }
                    }
                }
            }
        }

        if valid_staging {
            let odd_bytes = match tokio::fs::read(&staging).await {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("Failed to read double-sided staging file: {e}");
                    return false;
                }
            };
            let _ = tokio::fs::remove_file(&staging).await;

            match super::collate_double_sided(state, &odd_bytes, &bytes).await {
                Ok(collated) => {
                    tracing::info!("Collated double-sided scan for {}", path.display());
                    bytes = collated;
                    let stem = Path::new(&filename)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("document");
                    filename = format!("{stem}-collated.pdf");
                }
                Err(e) => {
                    tracing::error!("Collation failed: {e:?}");
                    return false;
                }
            }
        } else {
            if let Err(e) = tokio::fs::write(&staging, &bytes).await {
                tracing::warn!("Failed to stage double-sided scan: {e}");
                return false;
            }
            if state.config.consumer.delete_after_consume {
                let _ = tokio::fs::remove_file(path).await;
            }
            tracing::info!(
                "Received odd-numbered pages of double-sided scan, waiting up to 30 minutes for even-numbered pages"
            );
            return true;
        }
    }

    tracing::info!("Consuming {}", path.display());
    let req = ConsumeRequest {
        bytes,
        filename,
        source: "consume_folder".into(),
        ..Default::default()
    };
    match consume_and_wait(state, req).await {
        Ok(ids) => {
            tracing::info!("Consumed {} -> document(s) {:?}", path.display(), ids);
            if state.config.consumer.delete_after_consume {
                if let Err(e) = tokio::fs::remove_file(path).await {
                    tracing::warn!("Could not delete {} after consuming: {e}", path.display());
                }
            }
            true
        }
        Err(msg) => {
            tracing::warn!("Failed to consume {}: {msg}", path.display());
            false
        }
    }
}


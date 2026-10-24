//! Ingestion extras: TAG barcodes, sub-directories as tags and
//! double-sided collation.

use crate::state::AppState;
use base64::Engine;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

const STAGING_NAME: &str = "double-sided-staging.pdf";
const STAGING_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Find a tag by case-insensitive name or create it.
pub async fn get_or_create_tag(state: &AppState, name: &str) -> Option<i64> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    if let Ok(Some(id)) =
        sqlx::query_scalar::<_, i64>("SELECT id FROM tags WHERE name = $1 COLLATE NOCASE")
            .bind(name)
            .fetch_optional(&state.db)
            .await
    {
        return Some(id);
    }
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO tags (name) VALUES ($1) ON CONFLICT(name) DO UPDATE SET name = name RETURNING id",
    )
    .bind(name)
    .fetch_one(&state.db)
    .await
    .ok()
}

/// Convert a Python replacement template (`\g<1>`, `\1`) to the `regex` crate's syntax.
fn py_template(sub: &str) -> String {
    let g = regex::Regex::new(r"\\g<(\w+)>|\\(\d+)").unwrap();
    g.replace_all(&sub.replace('$', "$$"), |c: &regex::Captures| {
        format!("${{{}}}", c.get(1).or_else(|| c.get(2)).unwrap().as_str())
    })
    .into_owned()
}

fn compiled_mapping(state: &AppState) -> Vec<(regex::Regex, String)> {
    state
        .config
        .consumer
        .tag_barcode_mapping
        .iter()
        .filter_map(|(pat, sub)| {
            regex::Regex::new(&format!("(?i)^(?:{pat})"))
                .ok()
                .map(|r| (r, py_template(sub)))
        })
        .collect()
}

fn tag_name_for(mapping: &[(regex::Regex, String)], raw: &str) -> Option<String> {
    let raw = raw.trim();
    for (re, sub) in mapping {
        if re.is_match(raw) {
            let name = if sub.is_empty() {
                raw.to_string()
            } else {
                re.replace(raw, sub.as_str()).into_owned()
            };
            return Some(name).filter(|n| !n.trim().is_empty());
        }
    }
    None
}

fn barcode_value(b: &Value) -> &str {
    b.get("value").and_then(|v| v.as_str()).unwrap_or("")
}

/// Tag ids named by `TAG:`-style barcodes (comma separated values allowed).
pub async fn tags_from_barcodes(state: &AppState, barcodes: &[Value]) -> Vec<i64> {
    if !state.config.consumer.enable_tag_barcode {
        return vec![];
    }
    let mapping = compiled_mapping(state);
    let mut ids = Vec::new();
    for b in barcodes {
        for raw in barcode_value(b).split(',') {
            if let Some(name) = tag_name_for(&mapping, raw) {
                if let Some(id) = get_or_create_tag(state, &name).await {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
        }
    }
    ids
}

/// 1-based pages (excluding page 1) holding a tag barcode, when tag splitting is enabled.
pub fn tag_split_pages(state: &AppState, barcodes: &[Value]) -> Vec<i64> {
    let c = &state.config.consumer;
    if !(c.enable_tag_barcode && c.tag_barcode_split) {
        return vec![];
    }
    let mapping = compiled_mapping(state);
    let mut pages: Vec<i64> = barcodes
        .iter()
        .filter(|b| {
            barcode_value(b)
                .split(',')
                .any(|raw| tag_name_for(&mapping, raw).is_some())
        })
        .filter_map(|b| b.get("page").and_then(|p| p.as_i64()))
        .filter(|p| *p > 1)
        .collect();
    pages.sort_unstable();
    pages.dedup();
    pages
}

/// Tag ids for the sub-directories between the consume dir and `path`.
pub async fn subdir_tags(state: &AppState, path: &Path) -> Vec<i64> {
    let c = &state.config.consumer;
    if !c.subdirs_as_tags {
        return vec![];
    }
    let Ok(rel) = path.strip_prefix(&state.config.storage.consume_dir) else {
        return vec![];
    };
    let Some(parent) = rel.parent() else {
        return vec![];
    };
    let mut ids = Vec::new();
    for comp in parent.components() {
        let name = comp.as_os_str().to_string_lossy();
        if c.enable_collate_double_sided && name == c.collate_double_sided_subdir.as_str() {
            continue;
        }
        if let Some(id) = get_or_create_tag(state, &name).await {
            ids.push(id);
        }
    }
    ids
}

// ---------------------------------------------------------------------------
// double-sided collation
// ---------------------------------------------------------------------------

/// Output order as 1-based page indexes into the concatenation `[odd..., even...]`:
/// odd pages in order, interleaved with the even-page scan read back to front.
pub fn collate_order(odd: usize, even: usize) -> Option<Vec<usize>> {
    if even > odd {
        return None;
    }
    let mut out = Vec::with_capacity(odd + even);
    for i in 0..odd {
        out.push(i + 1);
        if i < even {
            out.push(odd + (even - i));
        }
    }
    Some(out)
}

fn staging_path(state: &AppState) -> PathBuf {
    state
        .config
        .storage
        .media_root
        .join("scratch")
        .join(STAGING_NAME)
}

async fn worker_json(
    state: &AppState,
    endpoint: &str,
    form: reqwest::multipart::Form,
) -> Result<Value, String> {
    let resp = state
        .worker_client
        .post(format!("{}{}", state.config.worker.url, endpoint))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("worker unreachable: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("worker returned HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

fn pdf_part(bytes: Vec<u8>) -> reqwest::multipart::Part {
    reqwest::multipart::Part::bytes(bytes)
        .file_name("part.pdf")
        .mime_str("application/pdf")
        .unwrap()
}

/// Run `/pdf/edit` with the given 1-based page order, returning the single output PDF.
async fn pdf_edit(state: &AppState, pdf: &[u8], order: &[usize]) -> Result<Vec<u8>, String> {
    let ops: Vec<Value> = order
        .iter()
        .map(|p| serde_json::json!({"page": p, "doc": 0}))
        .collect();
    let v = worker_json(
        state,
        "/pdf/edit",
        reqwest::multipart::Form::new()
            .part("file", pdf_part(pdf.to_vec()))
            .text("operations", serde_json::to_string(&ops).unwrap()),
    )
    .await?;
    let part = v
        .get("parts")
        .and_then(|p| p.as_array())
        .and_then(|a| a.first())
        .and_then(|s| s.as_str())
        .ok_or("worker returned no PDF")?;
    base64::engine::general_purpose::STANDARD
        .decode(part)
        .map_err(|e| e.to_string())
}

/// Page count via the worker: probe whether page `k` exists (exponential + binary search).
async fn pdf_pages(state: &AppState, pdf: &[u8]) -> Result<usize, String> {
    if pdf_edit(state, pdf, &[1]).await.is_err() {
        return Err("not a readable PDF".into());
    }
    let (mut lo, mut hi) = (1usize, 2usize); // lo exists
    while pdf_edit(state, pdf, &[hi]).await.is_ok() {
        lo = hi;
        hi *= 2;
        if hi > 4096 {
            break;
        }
    }
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if pdf_edit(state, pdf, &[mid]).await.is_ok() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(lo)
}

/// Collate the even-page scan `second` onto the odd-page scan `first` with the worker's
/// `/pdf/merge` and `/pdf/edit` endpoints.
pub async fn collate_pdfs(
    state: &AppState,
    first: Vec<u8>,
    second: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let n_first = pdf_pages(state, &first).await?;
    let n_second = pdf_pages(state, &second).await?;
    let order = collate_order(n_first, n_second).ok_or_else(|| {
        "This second file (even numbered pages) contains more pages than the first/odd \
         numbered one. This means the two uploaded files don't belong to the same \
         double-sided scan. Please retry, starting with the odd numbered pages again."
            .to_string()
    })?;
    let v = worker_json(
        state,
        "/pdf/merge",
        reqwest::multipart::Form::new()
            .part("files", pdf_part(first))
            .part("files", pdf_part(second)),
    )
    .await?;
    let merged = v
        .get("pdf_b64")
        .and_then(|x| x.as_str())
        .ok_or("worker returned no merged PDF")
        .map_err(String::from)
        .and_then(|s| {
            base64::engine::general_purpose::STANDARD
                .decode(s)
                .map_err(|e| e.to_string())
        })?;
    pdf_edit(state, &merged, &order).await
}

/// If `path` lives in the double-sided sub-directory, handle it and return `Some(success)`.
/// First file = odd pages (staged); second file = even pages (collated into a new file placed
/// in the consume dir without the `double-sided` path part, which the watcher then ingests).
/// Source files are removed here.
pub async fn maybe_collate(state: &AppState, path: &Path, bytes: &[u8]) -> Option<bool> {
    let c = &state.config.consumer;
    if !c.enable_collate_double_sided || c.collate_double_sided_subdir.is_empty() {
        return None;
    }
    let consume = &state.config.storage.consume_dir;
    let rel = path.strip_prefix(consume).ok()?;
    let subdir = c.collate_double_sided_subdir.as_str();
    let in_subdir = rel
        .parent()?
        .components()
        .any(|p| p.as_os_str().to_string_lossy() == subdir);
    if !in_subdir {
        return None;
    }
    if !bytes.starts_with(b"%PDF") {
        tracing::warn!(
            "Unsupported file type for collation of double-sided scans: {}",
            path.display()
        );
        return Some(false);
    }

    let staging = staging_path(state);
    if let Some(dir) = staging.parent() {
        let _ = tokio::fs::create_dir_all(dir).await;
    }
    let mut valid = false;
    if let Ok(meta) = tokio::fs::metadata(&staging).await {
        let age = meta
            .modified()
            .ok()
            .and_then(|m| m.elapsed().ok())
            .unwrap_or_default();
        if age > STAGING_TIMEOUT {
            tracing::warn!("Outdated double sided staging file exists, deleting it");
            let _ = tokio::fs::remove_file(&staging).await;
        } else {
            valid = true;
        }
    }

    if !valid {
        if tokio::fs::write(&staging, bytes).await.is_err() {
            return Some(false);
        }
        tracing::info!(
            "Got odd numbered pages of a double-sided scan, staged; waiting up to 30 minutes for the even pages"
        );
        let _ = tokio::fs::remove_file(path).await;
        return Some(true);
    }

    let first = tokio::fs::read(&staging).await.ok()?;
    let result = collate_pdfs(state, first, bytes.to_vec()).await;
    // Whatever happens the user must be able to restart from scratch.
    let _ = tokio::fs::remove_file(&staging).await;
    let _ = tokio::fs::remove_file(path).await;
    match result {
        Ok(merged) => {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("scan");
            let mut dest = consume.clone();
            for comp in rel.parent().unwrap_or(Path::new("")).components() {
                if comp.as_os_str().to_string_lossy() != subdir {
                    dest.push(comp);
                }
            }
            let _ = tokio::fs::create_dir_all(&dest).await;
            dest.push(format!("{stem}-collated.pdf"));
            let ok = tokio::fs::write(&dest, merged).await.is_ok();
            tracing::info!("Collated double-sided scan into {}", dest.display());
            Some(ok)
        }
        Err(e) => {
            tracing::warn!("Double-sided collation failed: {e}");
            Some(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_interleaves_reversed_even_pages() {
        assert_eq!(collate_order(3, 3).unwrap(), vec![1, 6, 2, 5, 3, 4]);
        assert_eq!(collate_order(3, 2).unwrap(), vec![1, 5, 2, 4, 3]);
        assert!(collate_order(2, 3).is_none());
    }

    #[test]
    fn python_templates() {
        assert_eq!(py_template(r"\g<1>"), "${1}");
        assert_eq!(py_template(r"x\1"), "x${1}");
    }
}

use crate::config::Config;
use crate::search::SearchFields;
use sqlx::SqlitePool;
use std::sync::Arc;
use tantivy::schema::Schema;
use tantivy::{Index, IndexReader, IndexWriter};
use tokio::sync::{broadcast, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub ws_tx: broadcast::Sender<String>,
    pub worker_client: reqwest::Client,
    pub index: Arc<Index>,
    pub index_writer: Arc<Mutex<IndexWriter>>,
    pub index_reader: IndexReader,
    #[allow(dead_code)]
    pub search_schema: Schema,
    pub search_fields: SearchFields,
}

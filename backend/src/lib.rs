//! FileDeck backend core library.
//!
//! FileDeck is a modern, local-first document archive and management engine written in Rust.
//! It integrates Tantivy for full-text search, SQLite for relational document storage,
//! a declarative matching engine for automated taxonomy tagging, and background ingestion.

pub mod ai;
pub mod config;
pub mod error;
pub mod history;
pub mod ingest;
pub mod matching;
pub mod routes;
pub mod search;
pub mod state;
pub mod workflows;

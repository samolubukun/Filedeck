//! Integration tests for FileDeck's configuration loader, TOML serialization, and schema models.

use filedeck_backend::config::{
    AiConfig, AppConfig, Config, ConsumerConfig, DatabaseConfig, EmailConfig, OcrConfig,
    StorageConfig, WorkerConfig,
};
use std::path::PathBuf;

#[test]
fn test_default_storage_config() {
    let storage = StorageConfig {
        media_root: PathBuf::from("./data/media"),
        consume_dir: PathBuf::from("./data/consume"),
        index_dir: PathBuf::from("./data/index"),
        share_link_bundle_dir: PathBuf::from("./data/share_bundles"),
    };

    assert_eq!(storage.media_root, PathBuf::from("./data/media"));
    assert_eq!(storage.consume_dir, PathBuf::from("./data/consume"));
    assert_eq!(storage.index_dir, PathBuf::from("./data/index"));
    assert_eq!(storage.share_link_bundle_dir, PathBuf::from("./data/share_bundles"));
}

#[test]
fn test_default_database_config() {
    let db = DatabaseConfig {
        url: "sqlite://data/filedeck.db".to_string(),
    };
    assert!(db.url.starts_with("sqlite://"));
}

#[test]
fn test_worker_and_ocr_config() {
    let worker = WorkerConfig {
        url: "http://127.0.0.1:8900".to_string(),
        secret: "test_secret_key_12345".to_string(),
    };
    assert_eq!(worker.url, "http://127.0.0.1:8900");

    let ocr = OcrConfig {
        language: "eng+deu+fra".to_string(),
        mode: "redo".to_string(),
    };
    assert_eq!(ocr.language, "eng+deu+fra");
    assert_eq!(ocr.mode, "redo");
}

#[test]
fn test_ai_config_defaults() {
    let ai = AiConfig {
        enabled: true,
        provider: "ollama".to_string(),
        model_path: "llama3".to_string(),
        embedding_model: "nomic-embed-text".to_string(),
        base_url: "http://127.0.0.1:11434".to_string(),
        api_key: "".to_string(),
        llm_model: "llama3:latest".to_string(),
    };

    assert!(ai.enabled);
    assert_eq!(ai.provider, "ollama");
    assert_eq!(ai.embedding_model, "nomic-embed-text");
}

#[test]
fn test_app_config() {
    let app = AppConfig {
        title: "FileDeck Test Suite".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8080,
        base_url: "http://localhost:8080".to_string(),
    };

    assert_eq!(app.title, "FileDeck Test Suite");
    assert_eq!(app.port, 8080);
}

#[test]
fn test_consumer_config() {
    let consumer = ConsumerConfig {
        watch: true,
        delete_after_consume: false,
        recursive: true,
        enable_barcodes: true,
        barcode_string: "PATCHT".to_string(),
        retain_split_pages: false,
        enable_asn_barcode: true,
        asn_barcode_prefix: "ASN".to_string(),
        ignore_patterns: vec!["*.tmp".to_string(), "~*".to_string(), ".DS_Store".to_string()],
        subdirs_as_tags: true,
        enable_collate_double_sided: false,
        collate_double_sided_subdir: "double-sided".to_string(),
        enable_tag_barcode: true,
        tag_barcode_mapping: vec![
            (r"^TAG:(\w+)$".to_string(), r"\1".to_string()),
            (r"^PROJECT:([A-Z0-9]+)$".to_string(), r"Project-$1".to_string()),
        ],
        tag_barcode_split: false,
    };

    assert!(consumer.watch);
    assert!(consumer.recursive);
    assert_eq!(consumer.ignore_patterns.len(), 3);
    assert_eq!(consumer.tag_barcode_mapping.len(), 2);
}

#[test]
fn test_toml_roundtrip_serialization() {
    let original = Config {
        storage: StorageConfig {
            media_root: PathBuf::from("./test_media"),
            consume_dir: PathBuf::from("./test_consume"),
            index_dir: PathBuf::from("./test_index"),
            share_link_bundle_dir: PathBuf::from("./test_shares"),
        },
        database: DatabaseConfig {
            url: "sqlite::memory:".to_string(),
        },
        worker: WorkerConfig {
            url: "http://localhost:8900".to_string(),
            secret: "s3cr3t".to_string(),
        },
        ocr: OcrConfig {
            language: "eng".to_string(),
            mode: "skip".to_string(),
        },
        ai: AiConfig {
            enabled: false,
            provider: "openai".to_string(),
            model_path: "".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-mock-key".to_string(),
            llm_model: "gpt-4o-mini".to_string(),
        },
        app: AppConfig {
            title: "FileDeck Instance".to_string(),
            host: "0.0.0.0".to_string(),
            port: 3000,
            base_url: "http://localhost:3000".to_string(),
        },
        consumer: ConsumerConfig {
            watch: false,
            delete_after_consume: true,
            recursive: false,
            enable_barcodes: false,
            barcode_string: "".to_string(),
            retain_split_pages: false,
            enable_asn_barcode: false,
            asn_barcode_prefix: "".to_string(),
            ignore_patterns: vec![],
            subdirs_as_tags: false,
            enable_collate_double_sided: false,
            collate_double_sided_subdir: "".to_string(),
            enable_tag_barcode: false,
            tag_barcode_mapping: vec![],
            tag_barcode_split: false,
        },
        email: EmailConfig {
            host: "".to_string(),
            port: 587,
            user: "".to_string(),
            password: "".to_string(),
            from: "".to_string(),
            use_tls: true,
            use_ssl: false,
        },
    };

    let toml_string = toml::to_string(&original).expect("TOML serialization succeeds");
    assert!(toml_string.contains("[storage]"));
    assert!(toml_string.contains("[database]"));
    assert!(toml_string.contains("[app]"));

    let restored: Config = toml::from_str(&toml_string).expect("TOML deserialization succeeds");
    assert_eq!(restored.app.title, "FileDeck Instance");
    assert_eq!(restored.app.port, 3000);
    assert_eq!(restored.database.url, "sqlite::memory:");
}

#[test]
fn test_json_roundtrip_serialization() {
    let conf = Config::load();
    let json_val = serde_json::to_value(&conf).expect("JSON serialization succeeds");
    assert!(json_val.get("storage").is_some());
    assert!(json_val.get("database").is_some());
    assert!(json_val.get("app").is_some());
}

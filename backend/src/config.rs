use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub media_root: PathBuf,
    pub consume_dir: PathBuf,
    pub index_dir: PathBuf,
    pub share_link_bundle_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub url: String,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    pub language: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    pub provider: String,
    pub model_path: String,
    pub embedding_model: String,
    pub base_url: String,
    pub api_key: String,
    pub llm_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub title: String,
    pub host: String,
    pub port: u16,
    pub base_url: String,
}

/// Ingestion behaviour configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    /// Watch `consume_dir` for new files.
    pub watch: bool,
    /// Delete the source file from `consume_dir` after it was ingested.
    pub delete_after_consume: bool,
    pub recursive: bool,
    pub enable_barcodes: bool,
    pub barcode_string: String,
    pub retain_split_pages: bool,
    pub enable_asn_barcode: bool,
    pub asn_barcode_prefix: String,
    pub ignore_patterns: Vec<String>,
    /// Use sub-directory names of `consume_dir` as tags.
    pub subdirs_as_tags: bool,
    pub enable_collate_double_sided: bool,
    pub collate_double_sided_subdir: String,
    pub enable_tag_barcode: bool,
    /// (regex, substitution) pairs, first match wins. Python-style `\g<1>` is accepted.
    pub tag_barcode_mapping: Vec<(String, String)>,
    pub tag_barcode_split: bool,
}

/// Outgoing mail (document e-mailing). `host` empty = not configured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub from: String,
    pub use_tls: bool,
    pub use_ssl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub storage: StorageConfig,
    pub database: DatabaseConfig,
    pub worker: WorkerConfig,
    pub ocr: OcrConfig,
    pub ai: AiConfig,
    pub app: AppConfig,
    pub consumer: ConsumerConfig,
    pub email: EmailConfig,
}

/// Layered lookup: environment variable > `filedeck.toml` > built-in default.
struct Layers {
    file: toml::Table,
}

impl Layers {
    fn open() -> Self {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(p) = std::env::var("FILEDECK_CONFIG") {
            candidates.push(PathBuf::from(p));
        }
        candidates.push(PathBuf::from("filedeck.toml"));
        candidates.push(PathBuf::from("../filedeck.toml"));
        candidates.push(dirs_fallback().join("filedeck").join("filedeck.toml"));

        for path in candidates {
            if let Ok(text) = std::fs::read_to_string(&path) {
                match text.parse::<toml::Table>() {
                    Ok(table) => {
                        eprintln!("Loaded configuration from {}", path.display());
                        return Self { file: table };
                    }
                    Err(e) => eprintln!("Ignoring invalid config {}: {e}", path.display()),
                }
            }
        }
        Self {
            file: toml::Table::new(),
        }
    }

    fn raw(&self, env: &str, section: &str, key: &str) -> Option<String> {
        if let Ok(v) = std::env::var(env) {
            return Some(v);
        }
        let value = self.file.get(section)?.as_table()?.get(key)?;
        match value {
            toml::Value::String(s) => Some(s.clone()),
            toml::Value::Boolean(b) => Some(b.to_string()),
            toml::Value::Integer(i) => Some(i.to_string()),
            toml::Value::Float(f) => Some(f.to_string()),
            toml::Value::Array(a) => Some(
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            _ => None,
        }
    }

    /// Ordered string->string table; from a JSON object in the env var or a TOML table.
    fn mapping(&self, env: &str, section: &str, key: &str, default: &[(&str, &str)]) -> Vec<(String, String)> {
        if let Ok(v) = std::env::var(env) {
            if let Ok(serde_json::Value::Object(m)) = serde_json::from_str::<serde_json::Value>(&v) {
                return m
                    .into_iter()
                    .map(|(k, v)| (k, v.as_str().unwrap_or_default().to_string()))
                    .collect();
            }
        }
        if let Some(t) = self
            .file
            .get(section)
            .and_then(|s| s.as_table())
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_table())
        {
            return t
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                .collect();
        }
        default
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    fn string(&self, env: &str, section: &str, key: &str, default: &str) -> String {
        self.raw(env, section, key)
            .unwrap_or_else(|| default.to_string())
    }

    fn boolean(&self, env: &str, section: &str, key: &str, default: bool) -> bool {
        self.raw(env, section, key)
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(default)
    }

    fn path(&self, env: &str, section: &str, key: &str, default: PathBuf) -> PathBuf {
        self.raw(env, section, key)
            .map(|v| expand_home(&v))
            .unwrap_or(default)
    }
}

fn expand_home(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/").or_else(|| p.strip_prefix("~\\")) {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_default();
        return home.join(rest);
    }
    PathBuf::from(p)
}

impl Config {
    pub fn load() -> Self {
        let l = Layers::open();
        let base_data_dir = dirs_fallback().join("filedeck");

        let media_root = l.path(
            "FILEDECK_MEDIA_ROOT",
            "storage",
            "media_root",
            base_data_dir.join("media"),
        );
        let consume_dir = l.path(
            "FILEDECK_CONSUME_DIR",
            "storage",
            "consume_dir",
            base_data_dir.join("consume"),
        );
        let index_dir = l.path(
            "FILEDECK_INDEX_DIR",
            "storage",
            "index_dir",
            base_data_dir.join("index"),
        );
        let share_link_bundle_dir = l.path(
            "FILEDECK_BUNDLE_DIR",
            "storage",
            "share_link_bundle_dir",
            base_data_dir.join("bundles"),
        );

        let default_db = format!(
            "sqlite://{}",
            base_data_dir
                .join("db.sqlite3")
                .to_string_lossy()
                .replace('\\', "/")
        );
        let db_url = match l.raw("FILEDECK_DATABASE_URL", "database", "url") {
            Some(url) => match url.strip_prefix("sqlite://") {
                Some(path) => format!(
                    "sqlite://{}",
                    expand_home(path).to_string_lossy().replace('\\', "/")
                ),
                None => url,
            },
            None => default_db,
        };

        let port: u16 = l
            .raw("FILEDECK_PORT", "app", "port")
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let ignore_default = ".DS_Store,Thumbs.db,desktop.ini,~$*,.*,*.tmp,*.part,*.crdownload";

        Self {
            storage: StorageConfig {
                media_root,
                consume_dir,
                index_dir,
                share_link_bundle_dir,
            },
            database: DatabaseConfig { url: db_url },
            worker: WorkerConfig {
                url: l.string(
                    "FILEDECK_WORKER_URL",
                    "worker",
                    "url",
                    "http://127.0.0.1:8900",
                ),
                secret: std::env::var("WORKER_SECRET")
                    .ok()
                    .or_else(|| l.raw("FILEDECK_WORKER_SECRET", "worker", "secret"))
                    .unwrap_or_default(),
            },
            ocr: OcrConfig {
                language: l.string("FILEDECK_OCR_LANGUAGE", "ocr", "language", "eng"),
                mode: l.string("FILEDECK_OCR_MODE", "ocr", "mode", "skip_noarchive"),
            },
            ai: AiConfig {
                enabled: l.boolean("FILEDECK_AI_ENABLED", "ai", "enabled", false),
                provider: l.string("FILEDECK_AI_PROVIDER", "ai", "provider", "local"),
                model_path: l.string("FILEDECK_AI_MODEL_PATH", "ai", "model_path", ""),
                embedding_model: l.string(
                    "FILEDECK_AI_EMBEDDING_MODEL",
                    "ai",
                    "embedding_model",
                    "all-MiniLM-L6-v2",
                ),
                base_url: l.string(
                    "FILEDECK_AI_BASE_URL",
                    "ai",
                    "base_url",
                    "http://localhost:11434/v1",
                ),
                api_key: l.string("FILEDECK_AI_API_KEY", "ai", "api_key", ""),
                llm_model: l.string("FILEDECK_AI_LLM_MODEL", "ai", "llm_model", "qwen2.5:1.5b"),
            },
            app: AppConfig {
                title: l.string("FILEDECK_APP_TITLE", "app", "title", "FileDeck"),
                host: l.string("FILEDECK_HOST", "app", "host", "0.0.0.0"),
                port,
                base_url: format!("http://localhost:{}", port),
            },
            consumer: ConsumerConfig {
                watch: l.boolean("FILEDECK_CONSUMER_WATCH", "consumer", "watch", true),
                delete_after_consume: l.boolean(
                    "FILEDECK_CONSUMER_DELETE",
                    "consumer",
                    "delete_after_consume",
                    true,
                ),
                recursive: l.boolean("FILEDECK_CONSUMER_RECURSIVE", "consumer", "recursive", true),
                enable_barcodes: l.boolean(
                    "FILEDECK_CONSUMER_ENABLE_BARCODES",
                    "consumer",
                    "enable_barcodes",
                    true,
                ),
                barcode_string: l.string(
                    "FILEDECK_CONSUMER_BARCODE_STRING",
                    "consumer",
                    "barcode_string",
                    "PATCHT",
                ),
                retain_split_pages: l.boolean(
                    "FILEDECK_CONSUMER_RETAIN_SPLIT_PAGES",
                    "consumer",
                    "retain_split_pages",
                    false,
                ),
                enable_asn_barcode: l.boolean(
                    "FILEDECK_CONSUMER_ENABLE_ASN_BARCODE",
                    "consumer",
                    "enable_asn_barcode",
                    true,
                ),
                asn_barcode_prefix: l.string(
                    "FILEDECK_CONSUMER_ASN_BARCODE_PREFIX",
                    "consumer",
                    "asn_barcode_prefix",
                    "ASN",
                ),
                ignore_patterns: l
                    .string(
                        "FILEDECK_CONSUMER_IGNORE_PATTERNS",
                        "consumer",
                        "ignore_patterns",
                        ignore_default,
                    )
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                subdirs_as_tags: l.boolean(
                    "FILEDECK_CONSUMER_SUBDIRS_AS_TAGS",
                    "consumer",
                    "subdirs_as_tags",
                    false,
                ),
                enable_collate_double_sided: l.boolean(
                    "FILEDECK_CONSUMER_ENABLE_COLLATE_DOUBLE_SIDED",
                    "consumer",
                    "enable_collate_double_sided",
                    false,
                ),
                collate_double_sided_subdir: l.string(
                    "FILEDECK_CONSUMER_COLLATE_DOUBLE_SIDED_SUBDIR",
                    "consumer",
                    "collate_double_sided_subdir",
                    "double-sided",
                ),
                enable_tag_barcode: l.boolean(
                    "FILEDECK_CONSUMER_ENABLE_TAG_BARCODE",
                    "consumer",
                    "enable_tag_barcode",
                    false,
                ),
                tag_barcode_mapping: l.mapping(
                    "FILEDECK_CONSUMER_TAG_BARCODE_MAPPING",
                    "consumer",
                    "tag_barcode_mapping",
                    &[("TAG:(.*)", r"\g<1>")],
                ),
                tag_barcode_split: l.boolean(
                    "FILEDECK_CONSUMER_TAG_BARCODE_SPLIT",
                    "consumer",
                    "tag_barcode_split",
                    false,
                ),
            },
            email: EmailConfig {
                host: l.string("FILEDECK_EMAIL_HOST", "email", "host", ""),
                port: l
                    .string("FILEDECK_EMAIL_PORT", "email", "port", "25")
                    .parse()
                    .unwrap_or(25),
                user: l.string("FILEDECK_EMAIL_USER", "email", "user", ""),
                password: l.string("FILEDECK_EMAIL_PASSWORD", "email", "password", ""),
                from: l.string("FILEDECK_EMAIL_FROM", "email", "from", ""),
                use_tls: l.boolean("FILEDECK_EMAIL_USE_TLS", "email", "use_tls", false),
                use_ssl: l.boolean("FILEDECK_EMAIL_USE_SSL", "email", "use_ssl", false),
            },
        }
    }
}

fn dirs_fallback() -> PathBuf {
    if let Ok(app_data) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(app_data)
    } else if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        PathBuf::from(home).join(".local").join("share")
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }
}

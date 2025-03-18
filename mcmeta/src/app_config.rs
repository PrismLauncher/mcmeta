use eyre::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StorageFormat {
    Json {
        path: String,
    },
    Database {
        connection_url: String,
        prefix: String,
    },
}

#[derive(Deserialize, Debug, Clone)]
pub struct GeneratedConfig {
    pub storage_format: StorageFormat,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UpstreamConfig {
    pub max_parallel_fetch_connections: usize,
    pub storage_format: StorageFormat,
    #[serde(default)]
    pub download: DownloadConfig,
    pub static_directory: String,
}

#[derive(Deserialize, Debug)]
pub struct DebugLogConfig {
    pub enable: bool,
    pub path: String,
    pub prefix: String,
    pub level: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct DownloadConfig {
    pub mojang: crate::upstream::mojang::DownloadConfig,
    pub forge: crate::upstream::forge::DownloadConfig,
}

#[derive(Deserialize, Debug)]
pub struct MetaConfig {
    pub bind_address: String,
    pub upstream: UpstreamConfig,
    pub generated: GeneratedConfig,
    pub debug_log: DebugLogConfig,
}

impl MetaConfig {
    pub fn from_config(path: &str) -> Result<Self> {
        let config = config::Config::builder()
            .set_default("bind_address", "127.0.0.1:8080")?
            .set_default("upstream.storage_format.type", "json")?
            .set_default("upstream.storage_format.path", "meta")?
            .set_default("upstream.max_parallel_fetch_connections", 20)?
            .set_default("upstream.static_directory", "static")?
            .set_default("generated.storage_format.type", "json")?
            .set_default("generated.storage_format.path", "generated")?
            .set_default("debug_log.enable", true)?
            .set_default("debug_log.path", "./logs")?
            .set_default("debug_log.prefix", "mcmeta.log")?
            .set_default("debug_log.level", "info")?
            // optionally add config from a file. this is optional though
            .add_source(config::File::from(std::path::Path::new(path)).required(false))
            // environment overrides file
            .add_source(config::Environment::with_prefix("mcmeta").separator("__"))
            .build()?;

        config
            .try_deserialize::<'_, Self>()
            .wrap_err("Failed to parse config")
    }
}

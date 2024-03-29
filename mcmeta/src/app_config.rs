use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StorageFormat {
    Json {
        meta_directory: String,
        generated_directory: String,
    },
    Database,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MetadataConfig {
    pub max_parallel_fetch_connections: usize,
    pub static_directory: String,
}

#[derive(Deserialize, Debug)]
pub struct DebugLogConfig {
    pub enable: bool,
    pub path: String,
    pub prefix: String,
    pub level: String,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub bind_address: String,
    pub storage_format: StorageFormat,
    pub metadata: MetadataConfig,
    pub debug_log: DebugLogConfig,
}

impl ServerConfig {
    pub fn from_config(path: &str) -> Result<Self> {
        let config = config::Config::builder()
            .set_default("bind_address", "127.0.0.1:8080")?
            .set_default("storage_format.type", "json")?
            .set_default("storage_format.meta_directory", "meta")?
            .set_default("storage_format.generated_directory", "generated")?
            .set_default("metadata.max_parallel_fetch_connections", 4)?
            .set_default("metadata.static_directory", "static")?
            .set_default("debug_log.enable", false)?
            .set_default("debug_log.path", "./logs")?
            .set_default("debug_log.prefix", "mcmeta.log")?
            .set_default("debug_log.level", "debug")?
            // optionally add config from a file. this is optional though
            .add_source(config::File::from(std::path::Path::new(path)).required(false))
            // environment overrides file
            .add_source(config::Environment::with_prefix("mcmeta").separator("__"))
            .build()?;

        config.try_deserialize::<'_, Self>().map_err(Into::into)
    }
}

use serde::Deserialize;

use crate::MetaMCError;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StorageFormat {
    Json { meta_directory: String },
    Database,
}

#[derive(Deserialize, Debug)]
pub struct MetadataConfig {
    pub max_parallel_fetch_connections: usize,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub bind_address: String,
    pub storage_format: StorageFormat,
    pub metadata: MetadataConfig,
}

impl ServerConfig {
    pub fn from_config() -> Result<Self, MetaMCError> {
        let config = config::Config::builder()
            .set_default("bind_address", "127.0.0.1:8080")?
            .set_default("storage_format.type", "json")?
            .set_default("storage_format.meta_directory", "meta")?
            .set_default("metadata.max_parallel_fetch_connections", 4)?
            .add_source(config::File::new(
                "config/settings",
                config::FileFormat::Json,
            ))
            .add_source(config::Environment::with_prefix("mcmeta").separator("__"))
            .build()?;

        config.try_deserialize::<'_, Self>().map_err(Into::into)
    }
}

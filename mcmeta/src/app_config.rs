use serde::Deserialize;

use crate::MetaMCError;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case", untagged)]
pub enum StorageFormat {
    Json { meta_directory: String },
    Database,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub bind_address: String,
    pub storage_format: StorageFormat,
}

impl ServerConfig {
    pub fn from_config() -> Result<Self, MetaMCError> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("MCMETA"))
            .set_default("bind_address", "127.0.0.1:8080")
            .unwrap()
            .set_default("storage_format", "json")
            .unwrap()
            .set_default("meta_directory", "meta")
            .unwrap()
            .build()?;

        config.try_deserialize::<'_, Self>().map_err(Into::into)
    }
}

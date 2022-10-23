use custom_error::custom_error;
use libmcmeta::models::mojang::{MojangVersionManifest, MinecraftVersion};
use serde::Deserialize;

custom_error! {pub MojangMetadataError
    Config { source: config::ConfigError } = "Error while reading config from environment",
    Request { source: reqwest::Error } = "Request error: {source}",
    Deserialization { source: serde_json::Error } = "Deserialization error: {source}",
}

fn default_download_url() -> String {
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json".to_string()
}

#[derive(Deserialize, Debug)]
struct DownloadConfig {
    #[serde(default = "default_download_url")]
    pub manifest_url: String,
}

impl DownloadConfig {
    fn from_config() -> Result<Self, MojangMetadataError> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("MCMETA_MOJANG"))
            .build()?;
        
            config.try_deserialize::<'_, Self>()
                .map_err(Into::into)
    }
}

pub async fn load_manifest() -> Result<MojangVersionManifest, MojangMetadataError> {
    let client = reqwest::Client::new();
    let config = DownloadConfig::from_config()?;

    let response = client
        .get(&config.manifest_url)
        .send()
        .await?
        .error_for_status()?
        .json::<MojangVersionManifest>()
        .await?;
    
    Ok(response)
}

pub async fn load_version_manifest(version_url: &str) -> Result<MinecraftVersion, MojangMetadataError> {
    let client = reqwest::Client::new();

    let response = client
        .get(version_url)
        .send()
        .await?
        .error_for_status()?
        .json::<MinecraftVersion>()
        .await?;
    
    Ok(response)
}
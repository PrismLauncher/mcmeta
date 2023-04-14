use libmcmeta::models::mojang::{MinecraftVersion, MojangVersionManifest};
use serde::Deserialize;
use serde_valid::Validate;
use tracing::debug;

use anyhow::Result;

use crate::download::errors::MetadataError;

fn default_download_url() -> String {
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json".to_string()
}

#[derive(Deserialize, Debug)]
struct DownloadConfig {
    #[serde(default = "default_download_url")]
    pub manifest_url: String,
}

impl DownloadConfig {
    fn from_config() -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("MCMETA_MOJANG"))
            .build()?;

        config.try_deserialize::<'_, Self>().map_err(Into::into)
    }
}

pub async fn load_manifest() -> Result<MojangVersionManifest> {
    let client = reqwest::Client::new();
    let config = DownloadConfig::from_config()?;

    let body = client
        .get(&config.manifest_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let manifest: MojangVersionManifest =
        serde_json::from_str(&body).map_err(|err| MetadataError::from_json_err(err, &body))?;
    manifest.validate()?;
    Ok(manifest)
}

pub async fn load_version_manifest(version_url: &str) -> Result<MinecraftVersion> {
    let client = reqwest::Client::new();

    debug!("Fetching version manifest from {:#?}", version_url);

    let body = client
        .get(version_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let manifest: MinecraftVersion =
        serde_json::from_str(&body).map_err(|err| MetadataError::from_json_err(err, &body))?;
    manifest.validate()?;
    Ok(manifest)
}

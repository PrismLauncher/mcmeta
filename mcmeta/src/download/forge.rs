use libmcmeta::models::forge::{ForgeMavenMetadata, ForgeMavenPromotions, ForgeVersionMeta};
use serde::Deserialize;
use serde_valid::Validate;
use tracing::debug;

use crate::download::errors::MetadataError;

use anyhow::Result;

fn default_maven_url() -> String {
    "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json".to_string()
}

fn default_promotions_url() -> String {
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json".to_string()
}

#[derive(Deserialize, Debug)]
struct DownloadConfig {
    #[serde(default = "default_maven_url")]
    pub maven_url: String,
    #[serde(default = "default_promotions_url")]
    pub promotions_url: String,
}

impl DownloadConfig {
    fn from_config() -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("MCMETA_FORGE"))
            .build()?;

        config.try_deserialize::<'_, Self>().map_err(Into::into)
    }
}

pub async fn load_maven_metadata() -> Result<ForgeMavenMetadata> {
    let client = reqwest::Client::new();
    let config = DownloadConfig::from_config()?;

    debug!(
        "Fetching forge maven manifest from {:#?}",
        &config.maven_url,
    );

    let body = client
        .get(&config.maven_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let metadata: ForgeMavenMetadata =
        serde_json::from_str(&body).map_err(|err| MetadataError::from_json_err(err, &body))?;
    metadata.validate()?;
    Ok(metadata)
}

pub async fn load_maven_promotions() -> Result<ForgeMavenPromotions> {
    let client = reqwest::Client::new();
    let config = DownloadConfig::from_config()?;

    debug!(
        "Fetching forge promotions manifest from {:#?}",
        &config.promotions_url,
    );

    let body = client
        .get(&config.promotions_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let promotions: ForgeMavenPromotions =
        serde_json::from_str(&body).map_err(|err| MetadataError::from_json_err(err, &body))?;
    promotions.validate()?;
    Ok(promotions)
}

pub async fn load_single_forge_files_manifest(url: &str) -> Result<ForgeVersionMeta> {
    let client = reqwest::Client::new();

    debug!("Fetching forge file manifest from {:#?}", url);

    let body = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let manifest: ForgeVersionMeta =
        serde_json::from_str(&body).map_err(|err| MetadataError::from_json_err(err, &body))?;
    manifest.validate()?;
    Ok(manifest)
}

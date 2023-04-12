use crate::utils::get_json_context_back;
use custom_error::custom_error;
use libmcmeta::models::mojang::{MinecraftVersion, MojangVersionManifest};
use serde::Deserialize;
use serde_valid::Validate;
use tracing::debug;

custom_error! {pub MojangMetadataError
    Config { source: config::ConfigError } = "Error while reading config from environment",
    Request { source: reqwest::Error } = "Request error: {source}",
    Deserialization { source: serde_json::Error } = "Deserialization error: {source}",
    BadData {
        ctx: String,
        source: serde_json::Error
    } = @{
        format!("{}. Context at {}:{} (may be truncated) \" {} \"", source, source.line(), source.column(), ctx)
    },
    Validation { source: serde_valid::validation::Errors } = "Validation Error: {source}",
}

impl MojangMetadataError {
    fn from_json_err(err: serde_json::Error, body: &str) -> Self {
        match err.classify() {
            serde_json::error::Category::Data => Self::BadData {
                ctx: get_json_context_back(&err, body, 200),
                source: err,
            },
            _ => err.into(),
        }
    }
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

        config.try_deserialize::<'_, Self>().map_err(Into::into)
    }
}

pub async fn load_manifest() -> Result<MojangVersionManifest, MojangMetadataError> {
    let client = reqwest::Client::new();
    let config = DownloadConfig::from_config()?;

    let body = client
        .get(&config.manifest_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let manifest: MojangVersionManifest = serde_json::from_str(&body)
        .map_err(|err| MojangMetadataError::from_json_err(err, &body))?;
    manifest.validate()?;
    Ok(manifest)
}

pub async fn load_version_manifest(
    version_url: &str,
) -> Result<MinecraftVersion, MojangMetadataError> {
    let client = reqwest::Client::new();

    debug!("Fetching version manifest from {:#?}", version_url);

    let body = client
        .get(version_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let manifest: MinecraftVersion = serde_json::from_str(&body)
        .map_err(|err| MojangMetadataError::from_json_err(err, &body))?;
    manifest.validate()?;
    Ok(manifest)
}

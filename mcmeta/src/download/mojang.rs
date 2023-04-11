use custom_error::custom_error;
use libmcmeta::models::mojang::{MinecraftVersion, MojangVersionManifest};
use serde::Deserialize;
use tracing::debug;

custom_error! {pub MojangMetadataError
    Config { source: config::ConfigError } = "Error while reading config from environment",
    Request { source: reqwest::Error } = "Request error: {source}",
    Deserialization { source: serde_json::Error } = "Deserialization error: {source}",
    BadData {
        ctx: String,
        line: usize,
        column: usize,
        source: serde_json::Error
    } = "{source}. Context at {line}:{column} (may be truncated) \" {ctx} \"" ,
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

    let manifest: MojangVersionManifest =
        serde_json::from_str(&body).map_err(|err| -> MojangMetadataError {
            match err.classify() {
                serde_json::error::Category::Data => {
                    let ctx_line = body.lines().nth(err.line() - 1).unwrap();
                    let line_pos = ctx_line.char_indices().nth(err.column()).unwrap().0;
                    let mut ctx = ctx_line.split_at(line_pos).1.to_owned();
                    let ctx_len = ctx.len();

                    if ctx_len > 100 {
                        ctx = ctx.split_at(100 - 4).0.to_owned() + " ...";
                    }

                    MojangMetadataError::BadData {
                        ctx,
                        line: err.line(),
                        column: err.column(),
                        source: err,
                    }
                    .into()
                }
                _ => err.into(),
            }
        })?;
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
    let manifest: MinecraftVersion =
        serde_json::from_str(&body).map_err(|err| -> MojangMetadataError {
            match err.classify() {
                serde_json::error::Category::Data => {
                    let ctx_line = body.lines().nth(err.line() - 1).unwrap();
                    let line_pos = ctx_line.char_indices().nth(err.column()).unwrap().0;
                    let mut ctx = ctx_line.split_at(line_pos).1.to_owned();
                    let ctx_len = ctx.len();

                    if ctx_len > 100 {
                        ctx = ctx.split_at(100 - 4).0.to_owned() + " ...";
                    }

                    MojangMetadataError::BadData {
                        ctx,
                        line: err.line(),
                        column: err.column(),
                        source: err,
                    }
                    .into()
                }
                _ => err.into(),
            }
        })?;
    Ok(manifest)
}

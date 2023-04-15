use futures::{stream, StreamExt};
use tracing::{debug, info, warn};

use anyhow::{Context, Result};

use crate::{app_config::MetadataConfig, download, storage::StorageFormat};

pub async fn initialize_forge_metadata(
    storage_format: &StorageFormat,
    metadata_cfg: &MetadataConfig,
) -> Result<()> {
    info!("Checking for Forge metadata");
    match storage_format {
        StorageFormat::Json {
            meta_directory,
            generated_directory: _,
        } => {
            return initialize_forge_metadata_json(metadata_cfg, meta_directory)
                .await
                .with_context(|| "Failed to initializ Forge metadata in the json format")
        }
        StorageFormat::Database => todo!(),
    }
}

async fn initialize_forge_metadata_json(
    metadata_cfg: &MetadataConfig,
    meta_directory: &str,
) -> Result<()> {
    let metadata_dir = std::path::Path::new(meta_directory);
    let forge_meta_dir = metadata_dir.join("forge");

    if !forge_meta_dir.exists() {
        info!(
            "Forge metadata directory at {} does not exist, creating it",
            forge_meta_dir.display()
        );
        std::fs::create_dir_all(&forge_meta_dir)?;
    }

    let main_metadata = download::forge::load_maven_metadata().await?;
    let promotions_metadata = download::forge::load_maven_promotions().await?;

    Ok(())
}

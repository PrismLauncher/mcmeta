use std::sync::Arc;

use crate::{app_config::MetadataConfig, app_config::StorageFormat};
use anyhow::Result;
use tracing::info;

mod forge;
mod mojang;

impl StorageFormat {
    pub async fn initialize_metadata(&self, metadata_cfg: &MetadataConfig) -> Result<()> {
        let updater = UpstreamMetadataUpdater {
            storage_format: Arc::new(self.clone()),
            metadata_cfg: Arc::new(metadata_cfg.clone()),
        };
        match self {
            StorageFormat::Json {
                meta_directory,
                generated_directory: _,
            } => {
                let metadata_dir = std::path::Path::new(meta_directory);
                if !metadata_dir.exists() {
                    info!(
                        "Metadata directory at {} does not exist, creating it",
                        meta_directory
                    );
                    std::fs::create_dir_all(metadata_dir)?;
                }
            }
            StorageFormat::Database => todo!(),
        }

        updater.initialize_mojang_metadata().await?;
        updater.initialize_forge_metadata().await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct UpstreamMetadataUpdater {
    storage_format: Arc<StorageFormat>,
    metadata_cfg: Arc<MetadataConfig>,
}

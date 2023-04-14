use tracing::info;

use crate::{app_config::MetadataConfig, app_config::StorageFormat, errors::MetaMCError};

mod mojang;

impl StorageFormat {
    pub async fn initialize_metadata(
        &self,
        metadata_cfg: &MetadataConfig,
    ) -> Result<(), MetaMCError> {
        match self {
            StorageFormat::Json { meta_directory } => {
                let metadata_dir = std::path::Path::new(meta_directory);
                if !metadata_dir.exists() {
                    info!(
                        "Metadata directory at {} does not exist, creating it",
                        meta_directory
                    );
                    std::fs::create_dir_all(metadata_dir)?;
                }

                mojang::initialize_mojang_metadata(&self, metadata_cfg).await?;
            }
            StorageFormat::Database => todo!(),
        }

        Ok(())
    }
}

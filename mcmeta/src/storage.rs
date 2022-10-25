use libmcmeta::models::mojang::MojangVersionManifest;
use tracing::info;

use crate::{app_config::StorageFormat, download, MetaMCError};

impl StorageFormat {
    pub async fn initialize_metadata(&self) -> Result<(), MetaMCError> {
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

                self.initialize_mojang_metadata().await?;
            }
            StorageFormat::Database => todo!(),
        }

        Ok(())
    }

    pub async fn initialize_mojang_metadata(&self) -> Result<(), MetaMCError> {
        match self {
            StorageFormat::Json { meta_directory } => {
                info!("Checking for Mojang metadata");
                let metadata_dir = std::path::Path::new(meta_directory);
                let mojang_meta_dir = metadata_dir.join("mojang");

                if !mojang_meta_dir.exists() {
                    info!(
                        "Mojang metadata directory at {} does not exist, creating it",
                        mojang_meta_dir.display()
                    );
                    std::fs::create_dir_all(&mojang_meta_dir)?;
                }

                let local_manifest = mojang_meta_dir.join("version_manifest_v2.json");
                if !local_manifest.exists() {
                    info!("Mojang metadata does not exist, downloading it");
                    let manifest = download::mojang::load_manifest().await?;
                    let manifest_json = serde_json::to_string_pretty(&manifest)?;
                    std::fs::write(&local_manifest, manifest_json)?;
                }
                let manifest = serde_json::from_str::<MojangVersionManifest>(
                    &std::fs::read_to_string(&local_manifest)?,
                )?;
                let versions_dir = mojang_meta_dir.join("versions");
                if !versions_dir.exists() {
                    info!(
                        "Mojang versions directory at {} does not exist, creating it",
                        versions_dir.display()
                    );
                    std::fs::create_dir_all(&versions_dir)?;
                }
                for version in &manifest.versions {
                    let version_file = versions_dir.join(format!("{}.json", &version.id));
                    if !version_file.exists() {
                        info!(
                            "Mojang metadata for version {} does not exist, downloading it",
                            &version.id
                        );
                        let version_manifest =
                            download::mojang::load_version_manifest(&version.url).await?;
                        let version_manifest_json =
                            serde_json::to_string_pretty(&version_manifest)?;
                        std::fs::write(&version_file, version_manifest_json)?;
                    }
                }
            }
            StorageFormat::Database => todo!(),
        }

        Ok(())
    }
}

use futures::{stream, StreamExt};
use libmcmeta::models::mojang::{MojangVersionManifest, MojangVersionManifestVersion};
use tracing::{debug, info, warn};

use custom_error::custom_error;

use crate::{app_config::MetadataConfig, app_config::StorageFormat, download, MetaMCError};

custom_error! {
    pub StorageError
    MetaMC { source: MetaMCError } = "{source}",
    Join { source: tokio::task::JoinError } = "Thread join error: {source}",
}

pub async fn initialize_mojang_version_manifest(
    versions_dir: std::path::PathBuf,
    version: MojangVersionManifestVersion,
) -> Result<(), MetaMCError> {
    let version_file = versions_dir.join(format!("{}.json", &version.id));
    if !version_file.exists() {
        info!(
            "Mojang metadata for version {} does not exist, downloading it",
            &version.id
        );
        let version_manifest = download::mojang::load_version_manifest(&version.url)
            .await
            .map_err(|err| {
                warn!(
                    "Error parsing manifest for version {}: {}",
                    &version.id,
                    err.to_string()
                );
                err
            })?;
        let version_manifest_json = serde_json::to_string_pretty(&version_manifest)?;
        std::fs::write(&version_file, version_manifest_json)?;
    }
    Ok(())
}

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

                self.initialize_mojang_metadata(metadata_cfg).await?;
            }
            StorageFormat::Database => todo!(),
        }

        Ok(())
    }

    pub async fn initialize_mojang_metadata(
        &self,
        metadata_cfg: &MetadataConfig,
    ) -> Result<(), MetaMCError> {
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

                let versions = manifest.versions;
                let tasks = stream::iter(versions)
                    .map(|version| {
                        let v = version.clone();
                        let dir = versions_dir.clone();
                        tokio::spawn(
                            async move { initialize_mojang_version_manifest(dir, v).await },
                        )
                    })
                    .buffer_unordered(metadata_cfg.max_parallel_fetch_connections);
                tasks
                    .map(|t| async {
                        match t {
                            Ok(Ok(t)) => Ok(t),
                            Ok(Err(e)) => Err(e),
                            Err(e) => Err(e.into()),
                        }
                    })
                    .for_each(|t| async {
                        match t.await {
                            Ok(_) => {}
                            Err(e) => debug!("Task had an error: {:?}", e),
                        }
                    })
                    .await
            }
            StorageFormat::Database => todo!(),
        }

        Ok(())
    }
}

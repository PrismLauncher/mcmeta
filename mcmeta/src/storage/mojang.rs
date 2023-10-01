use std::sync::Arc;

use futures::{stream, StreamExt};
use libmcmeta::models::mojang::{
    ExperimentEntry, ExperimentIndex, MinecraftVersion, MojangVersionManifest,
    MojangVersionManifestVersion, OldSnapshotEntry, OldSnapshotIndex, VersionDownload,
    VersionDownloads,
};
use tracing::{debug, info, warn};

use anyhow::{anyhow, Context, Result};

use crate::{
    download,
    storage::{StorageFormat, UpstreamMetadataUpdater},
    utils::process_results,
};

#[derive(Clone)]
pub struct MojangDataStorage {
    storage_format: Arc<StorageFormat>,
}

impl MojangDataStorage {
    pub fn meta_dir(&self) -> Result<std::path::PathBuf> {
        match *self.storage_format {
            StorageFormat::Json {
                ref meta_directory,
                generated_directory: _,
            } => {
                let metadata_dir = std::path::Path::new(&meta_directory);
                let mojang_meta_dir = metadata_dir.join("mojang");

                if !mojang_meta_dir.exists() {
                    info!(
                        "Mojang metadata directory at {} does not exist, creating it",
                        mojang_meta_dir.display()
                    );
                    std::fs::create_dir_all(&mojang_meta_dir)?;
                }
                Ok(mojang_meta_dir)
            }
            StorageFormat::Database => Err(anyhow!("Wrong storage format")),
        }
    }

    pub fn versions_dir(&self) -> Result<std::path::PathBuf> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let mojang_meta_dir = self.meta_dir()?;
                let versions_dir = mojang_meta_dir.join("versions");
                if !versions_dir.exists() {
                    let versions_dir = mojang_meta_dir.join("versions");
                    info!(
                        "Mojang versions directory at {} does not exist, creating it",
                        versions_dir.display()
                    );
                    std::fs::create_dir_all(&versions_dir)?;
                }
                Ok(versions_dir)
            }
            StorageFormat::Database => Err(anyhow!("Wrong storage format")),
        }
    }

    pub fn load_manifest(&self) -> Result<Option<MojangVersionManifest>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let local_manifest_path = self.meta_dir()?.join("version_manifest_v2.json");
                if local_manifest_path.is_file() {
                    let local_manifest = serde_json::from_str::<MojangVersionManifest>(
                        &std::fs::read_to_string(&local_manifest_path).with_context(|| {
                            format!(
                                "Failure reading file {}",
                                &local_manifest_path.to_string_lossy()
                            )
                        })?,
                    )?;
                    Ok(Some(local_manifest))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn store_manifest(&self, manifest: &MojangVersionManifest) -> Result<()> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let local_manifest_path = self.meta_dir()?.join("version_manifest_v2.json");
                let manifest_json = serde_json::to_string_pretty(&manifest)?;
                std::fs::write(&local_manifest_path, manifest_json).with_context(|| {
                    format!(
                        "Failure writing file {}",
                        local_manifest_path.to_string_lossy()
                    )
                })?;
                Ok(())
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn load_minecraft_version(&self, id: &str) -> Result<Option<MinecraftVersion>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let version_file = self.versions_dir()?.join(format!("{}.json", id));
                if version_file.is_file() {
                    let version = serde_json::from_str::<MinecraftVersion>(
                        &std::fs::read_to_string(&version_file).with_context(|| {
                            format!("Failure reading file {}", version_file.to_string_lossy())
                        })?,
                    )?;
                    Ok(Some(version))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn store_minecraft_version(&self, version: &MinecraftVersion) -> Result<()> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let version_file = self.versions_dir()?.join(format!("{}.json", version.id));
                let version_manifest_json = serde_json::to_string_pretty(&version)?;
                std::fs::write(&version_file, version_manifest_json).with_context(|| {
                    format!("Failure writing file {}", version_file.to_string_lossy())
                })?;
            }
            StorageFormat::Database => todo!(),
        }
        Ok(())
    }
}

impl UpstreamMetadataUpdater {
    pub async fn update_upstream_mojang(&self) -> Result<()> {
        info!("Checking for Mojang metadata");

        self.update_mojang_metadata()
            .await
            .with_context(|| "Failed to update Mojang metadata.")?;
        self.update_mojang_static_metadata()
            .await
            .with_context(|| "Failed to update Mojang static metadata.")?;
        Ok(())
    }

    pub async fn update_mojang_metadata(&self) -> Result<()> {
        use std::collections::{HashMap, HashSet};

        let local_storage = MojangDataStorage {
            storage_format: self.storage_format.clone(),
        };
        info!("Acquiring remote Mojang metadata");
        let remote_manifest = download::mojang::load_manifest().await?;
        let remote_versions: HashMap<String, MojangVersionManifestVersion> = HashMap::from_iter(
            remote_manifest
                .versions
                .iter()
                .map(|v| (v.id.clone(), v.clone())),
        );
        let remote_ids =
            HashSet::<String>::from_iter(remote_manifest.versions.iter().map(|v| v.id.clone()));

        let local_manifest = local_storage.load_manifest()?;
        let pending_ids: Vec<(String, bool)> = if let Some(local_manifest) = local_manifest {
            let local_versions: HashMap<String, MojangVersionManifestVersion> = HashMap::from_iter(
                local_manifest
                    .versions
                    .iter()
                    .map(|v| (v.id.clone(), v.clone())),
            );
            let local_ids =
                HashSet::<String>::from_iter(local_manifest.versions.iter().map(|v| v.id.clone()));

            let mut diff: Vec<(String, bool)> = remote_ids
                .difference(&local_ids)
                .cloned()
                .map(|id| (id, false))
                .collect();
            let mut out_of_date: Vec<(String, bool)> = local_ids
                .iter()
                .filter_map(|id| {
                    let remote_version = if let Some(rv) = remote_versions.get(id) {
                        rv
                    } else {
                        warn!("Mojang version {} does not exist remotely", id);
                        return None;
                    };

                    let local_version = local_versions
                        .get(id)
                        .expect("local version to exist locally");
                    if remote_version.time > local_version.time {
                        Some((id.clone(), true))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            diff.append(&mut out_of_date);
            diff
        } else {
            info!("Local Mojang metadata does not exist, fetching all versions");

            remote_ids.into_iter().map(|id| (id, true)).collect()
        };

        let tasks = stream::iter(pending_ids)
            .map(|(version, force_update)| {
                let ls = local_storage.clone();
                let v = remote_versions
                    .get(&version)
                    .expect("version to exist remotely")
                    .clone();
                tokio::spawn(async move {
                    update_mojang_version_manifest(&ls, &v, force_update)
                        .await
                        .with_context(|| format!("Failed to initialize Mojang version {}", v.id))
                })
            })
            .buffer_unordered(self.metadata_cfg.max_parallel_fetch_connections);
        let results = tasks
            .map(|t| match t {
                Ok(Ok(t)) => Ok(t),
                Ok(Err(e)) => {
                    debug!("Task had an error: {:?}", e);
                    Err(e)
                }
                Err(e) => {
                    debug!("Task had a Join error: {:?}", e);
                    Err(e.into())
                }
            })
            .collect::<Vec<_>>()
            .await;
        process_results(results)?;

        // update the locally stored manifest
        local_storage.store_manifest(&remote_manifest)?;
        Ok(())
    }

    pub async fn update_mojang_static_metadata(&self) -> Result<()> {
        let local_storage = MojangDataStorage {
            storage_format: self.storage_format.clone(),
        };

        let static_dir = std::path::Path::new(&self.metadata_cfg.static_directory);

        let static_experiments_path = static_dir.join("mojang").join("minecraft-experiments.json");
        if static_experiments_path.is_file() {
            let experiments = serde_json::from_str::<ExperimentIndex>(&std::fs::read_to_string(
                &static_experiments_path,
            )?)?;

            let tasks = stream::iter(experiments.experiments)
                .map(|experiment| {
                    let ls = local_storage.clone();
                    let e = experiment;

                    tokio::spawn(async move {
                        update_mojang_experiment(&ls, &e).await.with_context(|| {
                            format!("Failed to initialize Mojang experiment {}", e.id)
                        })
                    })
                })
                .buffer_unordered(self.metadata_cfg.max_parallel_fetch_connections);
            let results = tasks
                .map(|t| match t {
                    Ok(Ok(t)) => Ok(t),
                    Ok(Err(e)) => {
                        debug!("Task had an error: {:?}", e);
                        Err(e)
                    }
                    Err(e) => {
                        debug!("Task had a Join error: {:?}", e);
                        Err(e.into())
                    }
                })
                .collect::<Vec<_>>()
                .await;
            process_results(results)?;
        }

        let static_old_snapshots_path = static_dir
            .join("mojang")
            .join("minecraft-old-snapshots.json");
        if static_old_snapshots_path.is_file() {
            let old_snapshots = serde_json::from_str::<OldSnapshotIndex>(
                &std::fs::read_to_string(&static_old_snapshots_path)?,
            )?;

            let tasks = stream::iter(old_snapshots.old_snapshots)
                .map(|snapshot| {
                    let ls = local_storage.clone();
                    let s = snapshot;

                    tokio::spawn(async move {
                        update_mojang_old_snapshot(&ls, &s).await.with_context(|| {
                            format!("Failed to initialize Mojang experiment {}", s.id)
                        })
                    })
                })
                .buffer_unordered(self.metadata_cfg.max_parallel_fetch_connections);
            let results = tasks
                .map(|t| match t {
                    Ok(Ok(t)) => Ok(t),
                    Ok(Err(e)) => {
                        debug!("Task had an error: {:?}", e);
                        Err(e)
                    }
                    Err(e) => {
                        debug!("Task had a Join error: {:?}", e);
                        Err(e.into())
                    }
                })
                .collect::<Vec<_>>()
                .await;
            process_results(results)?;
        }

        Ok(())
    }
}

async fn update_mojang_version_manifest(
    local_storage: &MojangDataStorage,
    version: &MojangVersionManifestVersion,
    force_update: bool,
) -> Result<()> {
    let local_manifest = local_storage.load_minecraft_version(&version.id)?;
    if local_manifest.is_none() || force_update {
        info!(
            "Updating Mojang metadata for version {} to timestamp {}",
            &version.id, &version.time
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
        local_storage.store_minecraft_version(&version_manifest)?;
    }
    Ok(())
}

async fn update_mojang_experiment(
    local_storage: &MojangDataStorage,
    version: &ExperimentEntry,
) -> Result<()> {
    let local_version = local_storage.load_minecraft_version(&version.id)?;
    if local_version.is_none() {
        info!(
            "Mojang metadata for experiment {} does not exist, downloading it",
            &version.id
        );
        let version_manifest = download::mojang::load_zipped_version(&version.url)
            .await
            .map_err(|err| {
                warn!(
                    "Error parsing manifest for version {}: {}",
                    &version.id,
                    err.to_string()
                );
                err
            })?;
        local_storage.store_minecraft_version(&version_manifest)?;
    }
    Ok(())
}

async fn update_mojang_old_snapshot(
    local_storage: &MojangDataStorage,
    snapshot: &OldSnapshotEntry,
) -> Result<()> {
    let local_version = local_storage.load_minecraft_version(&snapshot.id)?;
    if local_version.is_none() {
        info!(
            "Mojang metadata for old snapshot {} does not exist, downloading it",
            &snapshot.id
        );

        let mut version_manifest = download::mojang::load_version_manifest(&snapshot.url)
            .await
            .map_err(|err| {
                warn!(
                    "Error parsing manifest for version {}: {}",
                    &snapshot.id,
                    err.to_string()
                );
                err
            })?;

        version_manifest.release_time = version_manifest.release_time.clone() + "T00:00:00+02:00";
        version_manifest.time = version_manifest.release_time.clone();

        version_manifest.downloads = Some(VersionDownloads {
            client: VersionDownload {
                url: snapshot.jar.clone(),
                sha1: snapshot.sha1.clone(),
                size: snapshot.size,
            },
            server: None,
            windows_server: None,
            client_mappings: None,
            server_mappings: None,
        });

        version_manifest.release_type = "old_snapshot".to_string();

        local_storage.store_minecraft_version(&version_manifest)?;
    }
    Ok(())
}

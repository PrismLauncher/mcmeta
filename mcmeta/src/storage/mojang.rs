use std::path::PathBuf;

use futures::{stream, StreamExt};
use libmcmeta::models::mojang::{
    ExperimentEntry, ExperimentIndex, MojangVersionManifest, MojangVersionManifestVersion,
    OldSnapshotEntry, OldSnapshotIndex, VersionDownload, VersionDownloads,
};
use tracing::{debug, error, info, warn};

use anyhow::{anyhow, Context, Result};

use crate::{app_config::MetadataConfig, download, storage::StorageFormat};

fn process_results<T>(results: Vec<Result<T>>) -> Result<Vec<T>> {
    let mut err_flag = false;
    let mut ok_results = vec![];
    for res in results {
        if let Ok(ok_res) = res {
            ok_results.push(ok_res);
        } else {
            error!("{}", res.err().unwrap());
            err_flag = true;
        }
    }
    if err_flag {
        Err(anyhow!("There were errors in the results"))
    } else {
        Ok(ok_results)
    }
}

pub async fn initialize_mojang_metadata(
    storage_format: &StorageFormat,
    metadata_cfg: &MetadataConfig,
) -> Result<()> {
    info!("Checking for Mojang metadata");
    match storage_format {
        StorageFormat::Json {
            meta_directory,
            generated_directory: _,
        } => {
            let metadata_dir = std::path::Path::new(meta_directory);
            let mojang_meta_dir = metadata_dir.join("mojang");

            if !mojang_meta_dir.exists() {
                info!(
                    "Mojang metadata directory at {} does not exist, creating it",
                    mojang_meta_dir.display()
                );
                std::fs::create_dir_all(&mojang_meta_dir)?;
            }

            update_mojang_metadata_json(metadata_cfg, &mojang_meta_dir)
                .await
                .with_context(|| "Failed to update Mojang metadata in the json storage format")?;
            update_mojang_static_metadata_json(metadata_cfg, &mojang_meta_dir)
                .await
                .with_context(|| {
                    "Failed to update Mojang static metadata in the json storage format."
                })
        }
        StorageFormat::Database => todo!(),
    }
}

async fn update_mojang_metadata_json(
    metadata_cfg: &MetadataConfig,
    mojang_meta_dir: &PathBuf,
) -> Result<()> {
    use std::collections::{HashMap, HashSet};

    info!("Acquiring remote Mojang metadata");
    let remote_manifest = download::mojang::load_manifest().await?;
    let remote_verions: HashMap<String, MojangVersionManifestVersion> = HashMap::from_iter(
        remote_manifest
            .versions
            .iter()
            .map(|v| (v.id.clone(), v.clone())),
    );
    let remote_ids =
        HashSet::<String>::from_iter(remote_manifest.versions.iter().map(|v| v.id.clone()));

    let local_manifest_path = mojang_meta_dir.join("version_manifest_v2.json");
    let pending_ids: Vec<String> = if !local_manifest_path.is_file() {
        info!("Local Mojang metadata does not exist, saving it");
        let manifest_json = serde_json::to_string_pretty(&remote_manifest)?;
        std::fs::write(&local_manifest_path, manifest_json)?;

        remote_ids.into_iter().collect()
    } else {
        let local_manifest = serde_json::from_str::<MojangVersionManifest>(
            &std::fs::read_to_string(&local_manifest_path)?,
        )?;
        let local_ids =
            HashSet::<String>::from_iter(local_manifest.versions.iter().map(|v| v.id.clone()));

        remote_ids
            .difference(&local_ids)
            .into_iter()
            .cloned()
            .collect()
    };

    let versions_dir = mojang_meta_dir.join("versions");
    if !versions_dir.exists() {
        let versions_dir = mojang_meta_dir.join("versions");
        info!(
            "Mojang versions directory at {} does not exist, creating it",
            versions_dir.display()
        );
        std::fs::create_dir_all(&versions_dir)?;
    }

    let tasks = stream::iter(pending_ids)
        .map(|version| {
            let v = remote_verions.get(&version).unwrap().clone();
            let dir = versions_dir.clone();
            tokio::spawn(async move {
                update_mojang_version_manifest_json(&dir, &v)
                    .await
                    .with_context(|| format!("Failed to initialize Mojang version {}", v.id))
            })
        })
        .buffer_unordered(metadata_cfg.max_parallel_fetch_connections);
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
    Ok(())
}

async fn update_mojang_static_metadata_json(
    metadata_cfg: &MetadataConfig,
    mojang_meta_dir: &PathBuf,
) -> Result<()> {
    let static_dir = std::path::Path::new(&metadata_cfg.static_directory);
    let versions_dir = mojang_meta_dir.join("versions");
    if !versions_dir.exists() {
        let versions_dir = mojang_meta_dir.join("versions");
        info!(
            "Mojang versions directory at {} does not exist, creating it",
            versions_dir.display()
        );
        std::fs::create_dir_all(&versions_dir)?;
    }

    let static_experiments_path = static_dir.join("mojang").join("minecraft-experiments.json");
    if static_experiments_path.is_file() {
        let experiments = serde_json::from_str::<ExperimentIndex>(&std::fs::read_to_string(
            &static_experiments_path,
        )?)?;

        let tasks = stream::iter(experiments.experiments)
            .map(|experiment| {
                let e = experiment.clone();
                let dir = versions_dir.clone();

                tokio::spawn(async move {
                    update_mojang_experiment_json(&dir, &e)
                        .await
                        .with_context(|| format!("Failed to initialize Mojang experiment {}", e.id))
                })
            })
            .buffer_unordered(metadata_cfg.max_parallel_fetch_connections);
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
        let old_snapshots = serde_json::from_str::<OldSnapshotIndex>(&std::fs::read_to_string(
            &static_old_snapshots_path,
        )?)?;

        let tasks = stream::iter(old_snapshots.old_snapshots)
            .map(|snapshot| {
                let s = snapshot.clone();
                let dir = versions_dir.clone();

                tokio::spawn(async move {
                    update_mojang_old_snapshot_json(&dir, &s)
                        .await
                        .with_context(|| format!("Failed to initialize Mojang experiment {}", s.id))
                })
            })
            .buffer_unordered(metadata_cfg.max_parallel_fetch_connections);
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

pub async fn update_mojang_version_manifest_json(
    versions_dir: &std::path::PathBuf,
    version: &MojangVersionManifestVersion,
) -> Result<()> {
    let version_file = versions_dir.join(format!("{}.json", &version.id));
    if !version_file.is_file() {
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

pub async fn update_mojang_experiment_json(
    versions_dir: &std::path::PathBuf,
    version: &ExperimentEntry,
) -> Result<()> {
    let version_file = versions_dir.join(format!("{}.json", &version.id));
    if !version_file.is_file() {
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
        let version_manifest_json = serde_json::to_string_pretty(&version_manifest)?;
        std::fs::write(&version_file, version_manifest_json)?;
    }
    Ok(())
}

pub async fn update_mojang_old_snapshot_json(
    versions_dir: &std::path::PathBuf,
    snapshot: &OldSnapshotEntry,
) -> Result<()> {
    let version_file = versions_dir.join(format!("{}.json", &snapshot.id));
    if !version_file.is_file() {
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

        let version_manifest_json = serde_json::to_string_pretty(&version_manifest)?;
        std::fs::write(&version_file, version_manifest_json)?;
    }
    Ok(())
}

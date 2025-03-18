use std::sync::Arc;

use libmcmeta::models::mojang::{
    ExperimentEntry, ExperimentIndex, MinecraftVersion, MojangVersionManifest,
    MojangVersionManifestVersion, OldSnapshotEntry, OldSnapshotIndex, VersionDownload,
    VersionDownloads,
};
use serde::Deserialize;
use serde_valid::Validate;
use tempdir::TempDir;
use tracing::{debug, error, info, info_span, instrument, warn};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use eyre::{Context, Result};

use crate::{
    app_config::UpstreamConfig,
    errors::MetadataError,
    storage::{ResourcePath, Storage, StorageImpl},
    upstream::download_binary_file,
    utils::deserialize_json_with_error_path,
};

fn default_download_url() -> String {
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json".to_string()
}

#[derive(Deserialize, Debug, Clone)]
pub struct DownloadConfig {
    #[serde(default = "default_download_url")]
    pub manifest_url: String,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        DownloadConfig {
            manifest_url: default_download_url(),
        }
    }
}

#[derive(Clone)]
pub struct MojangUpdater {
    storage: Arc<StorageImpl>,
    config: Arc<UpstreamConfig>,
    client: Arc<reqwest::Client>,
}

impl MojangUpdater {
    pub fn new(storage: Arc<StorageImpl>, config: UpstreamConfig) -> Self {
        MojangUpdater {
            storage,
            config: Arc::new(config),
            client: Arc::new(reqwest::Client::new()),
        }
    }

    #[instrument(skip(self))]
    pub async fn update(&self) -> Result<()> {
        info!("Checking for Mojang metadata");

        self.update_mojang_metadata()
            .await
            .wrap_err_with(|| "Failed to update Mojang metadata.")?;
        self.update_mojang_static_metadata()
            .await
            .wrap_err_with(|| "Failed to update Mojang static metadata.")?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn download_manifest(&self) -> Result<MojangVersionManifest> {
        let config = &self.config.download.mojang;

        info!(
            "Fetching minecraft client manifest from {:#?}",
            &config.manifest_url
        );

        let body = self
            .client
            .get(&config.manifest_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let manifest: MojangVersionManifest = deserialize_json_with_error_path(&body)
            .wrap_err("Failed to parse Mojang version manifest")?;
        manifest.validate()?;
        Ok(manifest)
    }

    #[instrument(skip(self))]
    pub async fn download_version_manifest(&self, version_url: &str) -> Result<MinecraftVersion> {
        info!(
            "Fetching minecraft version manifest from {:#?}",
            version_url
        );

        let body = self
            .client
            .get(version_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let manifest: MinecraftVersion =
            deserialize_json_with_error_path(&body).wrap_err_with(|| {
                format!(
                    "Failed to parse Mojang version manifest from: '{}'",
                    version_url
                )
            })?;
        manifest.validate()?;
        Ok(manifest)
    }

    #[instrument(skip(self))]
    pub async fn download_zipped_version(&self, version_url: &str) -> Result<MinecraftVersion> {
        use std::io::Read;

        info!("Fetching zipped version from {:#?}", version_url);

        let tmp_dir = TempDir::new("mcmeta_mojang_zip")?;
        let dest_path = {
            let url = reqwest::Url::parse(version_url)?;
            let fname = url
                .path_segments()
                .and_then(|segments| segments.last())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .unwrap_or("tmp.zip");

            tmp_dir.path().join(fname)
        };

        download_binary_file(&self.client, &dest_path, version_url).await?;

        let file = std::fs::File::open(&dest_path)?;

        let mut archive = zip::ZipArchive::new(file)?;

        let mut manifest: Option<MinecraftVersion> = None;
        for i in 0..archive.len() {
            let mut zfile = archive.by_index(i)?;
            if zfile.name().ends_with(".json") {
                debug!("Found {} as version json", zfile.name());
                let mut contents = String::new();
                zfile.read_to_string(&mut contents)?;

                manifest = Some(deserialize_json_with_error_path(&contents).wrap_err_with(||{
                    format!("Failed to parse zipped Mojang Minecraft version manifest '{}' in archive from '{}'", zfile.name(), version_url)
                })?);
            }
        }

        manifest.ok_or_else(|| {
            MetadataError::MissingMojangVersionManifest(version_url.to_string()).into()
        })
    }

    pub fn root_path(&self) -> ResourcePath {
        ResourcePath::new(&["mojang".to_string()])
    }

    pub fn versions_path(&self) -> ResourcePath {
        self.root_path().join("versions")
    }

    pub async fn load_manifest(&self) -> Result<Option<MojangVersionManifest>> {
        let record = self
            .storage
            .fetch_record(self.root_path(), "version_manifest_v2")
            .await?;
        Ok(record)
    }

    pub async fn store_manifest(&self, manifest: &MojangVersionManifest) -> Result<()> {
        self.storage
            .store_record(self.root_path(), "version_manifest_v2", manifest)
            .await?;
        Ok(())
    }

    pub async fn load_minecraft_version(&self, id: &str) -> Result<Option<MinecraftVersion>> {
        self.storage
            .fetch_record(self.versions_path(), id)
            .await
            .wrap_err_with(|| "Failed to load Minecraft Version")
    }

    pub async fn store_minecraft_version(&self, version: &MinecraftVersion) -> Result<()> {
        self.storage
            .store_record(self.versions_path(), &version.id, &version)
            .await
            .wrap_err_with(|| "Failed to store Minecraft Version")
    }

    #[instrument(skip(self))]
    pub async fn update_mojang_metadata(&self) -> Result<()> {
        use std::collections::{HashMap, HashSet};

        info!("Acquiring remote Mojang metadata");
        let remote_manifest = self.download_manifest().await?;
        let remote_versions: HashMap<String, MojangVersionManifestVersion> = HashMap::from_iter(
            remote_manifest
                .versions
                .iter()
                .map(|v| (v.id.clone(), v.clone())),
        );
        let remote_ids =
            HashSet::<String>::from_iter(remote_manifest.versions.iter().map(|v| v.id.clone()));

        let local_manifest = self.load_manifest().await?;
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
                    if remote_version.time > local_version.time
                        || remote_version.sha1 != local_version.sha1
                    {
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

        use futures::StreamExt;

        let task_span = info_span!("download_mojang_versions");
        task_span.pb_set_style(&super::progress_bar());
        task_span.pb_set_message("Updating Mojang Versions ... ");
        task_span.pb_set_length(pending_ids.len().try_into().unwrap_or_default());

        let task_span_entert = task_span.enter();

        let results = {
            futures::stream::iter(pending_ids)
                .map(|(version, force_update)| {
                    let ls = self.clone();
                    let v = remote_versions
                        .get(&version)
                        .expect("version to exist remotely")
                        .clone();
                    let spn = task_span.clone();
                    tokio::spawn(async move {
                        let res = ls
                            .update_mojang_version_manifest(&v, force_update)
                            .await
                            .wrap_err_with(|| {
                                format!("Failed to initialize Mojang version {}", v.id)
                            });
                        spn.pb_inc(1);
                        res
                    })
                })
                .buffer_unordered(self.config.max_parallel_fetch_connections)
                .map(|t| match t {
                    Ok(Ok(t)) => Ok(t),
                    Ok(Err(e)) => Err(e).wrap_err("Task had an error"),
                    Err(e) => Err::<(), eyre::Report>(e.into()).wrap_err("Task had a Join error"),
                })
                .collect::<Vec<_>>()
                .await
        };

        std::mem::drop(task_span_entert);
        std::mem::drop(task_span);

        let (_, failures) = crate::utils::process_results(results);
        if !failures.is_empty() {
            Err(MetadataError::BulkProcessingError(
                "Updating Mojang Metadata".to_string(),
                failures,
            )
            .into())
        } else {
            // update the locally stored manifest
            self.store_manifest(&remote_manifest).await?;
            Ok(())
        }
    }

    #[instrument(skip(self, version), fields(version = &version.id))]
    async fn update_mojang_version_manifest(
        &self,
        version: &MojangVersionManifestVersion,
        force_update: bool,
    ) -> Result<()> {
        let local_manifest = self.load_minecraft_version(&version.id).await?;
        if local_manifest.is_none() || force_update {
            info!(
                "Updating Mojang metadata for version {} to timestamp {}",
                &version.id, &version.time
            );
            let version_manifest = self
                .download_version_manifest(&version.url)
                .await
                .inspect_err(|err| {
                    warn!(
                        "Error parsing manifest for version {}: {}",
                        &version.id, err
                    )
                })?;
            self.store_minecraft_version(&version_manifest).await?;
        }
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn update_mojang_static_metadata(&self) -> Result<()> {
        let static_dir = std::path::Path::new(&self.config.static_directory);

        let static_experiments_path = static_dir.join("mojang").join("minecraft-experiments.json");
        if static_experiments_path.is_file() {
            let experiments = crate::utils::deserialize_json_with_error_path::<ExperimentIndex>(
                &std::fs::read_to_string(&static_experiments_path)?,
            )?;

            let task_span = info_span!("download_mojang_expriments");
            task_span.pb_set_style(&super::progress_bar());
            task_span.pb_set_message("Updating Mojang Experiments");
            task_span.pb_set_length(experiments.experiments.len().try_into().unwrap_or_default());

            let task_span_enter = task_span.enter();

            use futures::StreamExt;
            let tasks = futures::stream::iter(experiments.experiments)
                .map(|experiment| {
                    let ls = self.clone();
                    let e = experiment;
                    let spn = task_span.clone();

                    tokio::spawn(async move {
                        let res = ls.update_mojang_experiment(&e).await.wrap_err_with(|| {
                            format!("Failed to initialize Mojang experiment {}", e.id)
                        });
                        spn.pb_inc(1);
                        res
                    })
                })
                .buffer_unordered(self.config.max_parallel_fetch_connections);
            let results: Vec<Result<(), eyre::Error>> = tasks
                .map(|t| -> Result<()> {
                    match t {
                        Ok(Ok(_)) => Ok(()),
                        Ok(Err(e)) => Err(e).wrap_err("Task had an error"),
                        Err(e) => {
                            Err::<(), eyre::Report>(e.into()).wrap_err("Task had a Join error")
                        }
                    }
                })
                .collect()
                .await;

            std::mem::drop(task_span_enter);
            std::mem::drop(task_span);

            let (_, errs) = crate::utils::process_results::<(), eyre::Report>(results);
            if !errs.is_empty() {
                return Err(MetadataError::BulkProcessingError(
                    "Updating Mojang Experiment Metadata".to_string(),
                    errs,
                )
                .into());
            }
        }

        let static_old_snapshots_path = static_dir
            .join("mojang")
            .join("minecraft-old-snapshots.json");
        if static_old_snapshots_path.is_file() {
            let old_snapshots = crate::utils::deserialize_json_with_error_path::<OldSnapshotIndex>(
                &std::fs::read_to_string(&static_old_snapshots_path)?,
            )?;

            let task_span = info_span!("download_mojang_old_snapshots");
            task_span.pb_set_style(&super::progress_bar());
            task_span.pb_set_message("Updating Mojang Old Snapshots");
            task_span.pb_set_length(
                old_snapshots
                    .old_snapshots
                    .len()
                    .try_into()
                    .unwrap_or_default(),
            );

            let task_span_enter = task_span.enter();
            use futures::StreamExt;
            let tasks = futures::stream::iter(old_snapshots.old_snapshots)
                .map(|snapshot| {
                    let ls = self.clone();
                    let s = snapshot;
                    let spn = task_span.clone();
                    tokio::spawn(async move {
                        let res = ls.update_mojang_old_snapshot(&s).await.wrap_err_with(|| {
                            format!("Failed to initialize Mojang experiment {}", s.id)
                        });
                        spn.pb_inc(1);
                        res
                    })
                })
                .buffer_unordered(self.config.max_parallel_fetch_connections);
            let results = tasks
                .map(|t| match t {
                    Ok(Ok(t)) => Ok(t),
                    Ok(Err(e)) => {
                        error!("Task had an error: {:?}", e);
                        Err(e)
                    }
                    Err(e) => {
                        error!("Task had a Join error: {:?}", e);
                        Err(e.into())
                    }
                })
                .collect::<Vec<_>>()
                .await;

            std::mem::drop(task_span_enter);
            std::mem::drop(task_span);

            let (_, errs) = crate::utils::process_results::<(), eyre::Report>(results);
            if !errs.is_empty() {
                return Err(MetadataError::BulkProcessingError(
                    "Updating Mojang Old Snapshot Metadata".to_string(),
                    errs,
                )
                .into());
            }
        }
        Ok(())
    }

    #[instrument(skip(self, version), fields(version = &version.id))]
    async fn update_mojang_experiment(&self, version: &ExperimentEntry) -> Result<()> {
        let local_version = self.load_minecraft_version(&version.id).await?;
        if local_version.is_none() {
            info!(
                "Mojang metadata for experiment {} does not exist, downloading it",
                &version.id
            );
            let version_manifest = self
                .download_zipped_version(&version.url)
                .await
                .inspect_err(|err| {
                    warn!(
                        "Error parsing manifest for version {}: {}",
                        &version.id, err
                    );
                })?;
            self.store_minecraft_version(&version_manifest).await?;
        }
        Ok(())
    }

    #[instrument(skip(self, snapshot), fields(snapshot = &snapshot.id))]
    async fn update_mojang_old_snapshot(&self, snapshot: &OldSnapshotEntry) -> Result<()> {
        let local_version = self.load_minecraft_version(&snapshot.id).await?;
        if local_version.is_none() {
            info!(
                "Mojang metadata for old snapshot {} does not exist, downloading it",
                &snapshot.id
            );

            let mut version_manifest = self
                .download_version_manifest(&snapshot.url)
                .await
                .inspect_err(|err| {
                    warn!(
                        "Error parsing manifest for version {}: {}",
                        &snapshot.id, err
                    );
                })?;

            version_manifest.release_time =
                version_manifest.release_time.clone() + "T00:00:00+02:00";
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

            self.store_minecraft_version(&version_manifest).await?;
        }
        Ok(())
    }
}

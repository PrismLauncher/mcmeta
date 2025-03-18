use std::sync::Arc;

use chrono::TimeZone;
use eyre::{eyre, Context, Result};
use futures::{stream, StreamExt};
use serde::Deserialize;
use serde_valid::Validate;
use std::collections::{BTreeMap, HashSet};
use tempdir::TempDir;
use tracing::{debug, error, info, info_span, instrument, warn};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::{
    app_config::UpstreamConfig,
    errors::MetadataError,
    storage::{ResourcePath, Storage, StorageImpl},
    upstream,
    utils::{self, filehash, process_results_ok, HashAlgo},
};
use libmcmeta::models::{
    forge::{
        DerivedForgeIndex, ForgeEntry, ForgeFile, ForgeInstallerProfile, ForgeLegacyInfo,
        ForgeLegacyInfoList, ForgeMCVersionInfo, ForgeMavenMetadata, ForgeMavenPromotions,
        ForgeProcessedVersion, ForgeVersionMeta, InstallerInfo,
    },
    mojang::MojangVersion,
    MetaIndexEntry,
};

lazy_static! {
    pub static ref BAD_FORGE_VERSIONS: Vec<&'static str> = vec!["1.12.2-14.23.5.2851"];
}

fn default_maven_url() -> String {
    "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json".to_string()
}

fn default_promotions_url() -> String {
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json".to_string()
}

#[derive(Deserialize, Debug, Clone)]
pub struct DownloadConfig {
    #[serde(default = "default_maven_url")]
    pub maven_url: String,
    #[serde(default = "default_promotions_url")]
    pub promotions_url: String,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        DownloadConfig {
            maven_url: default_maven_url(),
            promotions_url: default_promotions_url(),
        }
    }
}

impl DownloadConfig {}

#[derive(Clone)]
pub struct ForgeUpdater {
    storage: Arc<StorageImpl>,
    config: Arc<UpstreamConfig>,
    client: Arc<reqwest::Client>,
}

impl ForgeUpdater {
    pub fn new(storage: Arc<StorageImpl>, config: UpstreamConfig) -> Self {
        ForgeUpdater {
            storage,
            config: Arc::new(config),
            client: Arc::new(reqwest::Client::new()),
        }
    }

    #[instrument(skip(self))]
    pub async fn update(&self) -> Result<()> {
        info!("Checking for Forge metadata");
        self.update_forge_metadata()
            .await
            .wrap_err_with(|| "Failed to update Forge metadata.")?;

        self.update_forge_installer_metadata()
            .await
            .wrap_err_with(|| "Failed to update Forge legacy metadata.")?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn download_maven_metadata(&self) -> Result<ForgeMavenMetadata> {
        let config = &self.config.download.forge;

        info!(
            "Fetching forge maven manifest from {:#?}",
            &config.maven_url,
        );

        let body = self
            .client
            .get(&config.maven_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let metadata: ForgeMavenMetadata = crate::utils::deserialize_json_with_error_path(&body)
            .wrap_err("Failed to parse forge maven metadata json")?;
        metadata.validate()?;
        Ok(metadata)
    }

    #[instrument(skip(self))]
    pub async fn download_maven_promotions(&self) -> Result<ForgeMavenPromotions> {
        let config = &self.config.download.forge;

        info!(
            "Fetching forge promotions manifest from {:#?}",
            &config.promotions_url,
        );

        let body = self
            .client
            .get(&config.promotions_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let promotions: ForgeMavenPromotions =
            crate::utils::deserialize_json_with_error_path(&body)
                .wrap_err("Failed to parse forge maven promotions json")?;
        promotions.validate()?;
        Ok(promotions)
    }

    pub async fn load_single_forge_files_manifest(&self, url: &str) -> Result<ForgeVersionMeta> {
        info!("Fetching forge file manifest from {:#?}", url);

        let body = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let manifest: ForgeVersionMeta = utils::deserialize_json_with_error_path(&body)
            .wrap_err_with(|| {
                format!(
                    "Failed to parse Forge version manifest json from : '{}'",
                    url
                )
            })?;
        manifest.validate()?;
        Ok(manifest)
    }

    #[instrument(skip(self))]
    pub async fn update_forge_metadata(&self) -> Result<()> {
        let maven_metadata = self.download_maven_metadata().await?;
        let promotions_metadata = self.download_maven_promotions().await?;

        let promoted_key_expression = regex::Regex::new(
            "(?P<mc>[^-]+)-(?P<promotion>(latest)|(recommended))(-(?P<branch>[a-zA-Z0-9\\.]+))?",
        )
        .expect("Promotion regex must compile");

        let mut recommended_set = HashSet::new();

        // FIXME: does not fully validate that the file has not changed format
        // NOTE: For some insane reason, the format of the versions here is special. It having a branch at the end means it
        //           affects that particular branch.
        //       We don't care about Forge having branches.
        //       Therefore we only use the short version part for later identification and filter out the branch-specific
        //           promotions (among other errors).
        info!("Processing Forge Promotions");

        let task_span = info_span!("process_forge_promos");
        task_span.pb_set_style(&super::progress_bar());
        task_span.pb_set_message("Processing Forge Promotions ...");
        task_span.pb_set_length(
            promotions_metadata
                .promos
                .len()
                .try_into()
                .unwrap_or_default(),
        );

        let task_span_enter = task_span.enter();

        for (promo_key, shortversion) in &promotions_metadata.promos {
            task_span.pb_inc(1);
            match promoted_key_expression.captures(promo_key) {
                None => {
                    warn!("Skipping promotion {}, the key did not parse:", promo_key);
                }
                Some(captures) => {
                    if captures.name("mc").is_none() {
                        debug!(
                            "Skipping promotion {}, because it has no Minecraft version.",
                            promo_key
                        );
                        continue;
                    }
                    if captures.name("branch").is_some() {
                        debug!(
                            "Skipping promotion {}, because it on a branch only.",
                            promo_key
                        );
                        continue;
                    } else if let Some(promotion) = captures.name("promotion") {
                        if promotion.as_str() == "recommended" {
                            recommended_set.insert(shortversion.clone());
                            debug!("forge {} added to recommended set", &shortversion);
                        } else if promotion.as_str() == "latest" {
                            continue;
                        }
                    } else {
                        warn!("Unknown capture state {:?}", captures);
                    }
                }
            }
        }

        std::mem::drop(task_span_enter);
        std::mem::drop(task_span);

        info!("Processing Forge Versions");

        let task_span = info_span!("process_forge_maven_versions");
        task_span.pb_set_style(&super::progress_bar());
        task_span.pb_set_message("Processing Forge Maven ...");
        task_span.pb_set_length(maven_metadata.versions.len().try_into().unwrap_or_default());

        let task_span_enter = task_span.enter();

        let remote_forge_version_pairs =
            HashSet::<(String, String)>::from_iter(maven_metadata.versions.iter().flat_map(
                |(mc_version, forge_version_list)| {
                    task_span.pb_inc(1);
                    forge_version_list
                        .iter()
                        .map(|forge_version| (mc_version.clone(), forge_version.clone()))
                },
            ));

        std::mem::drop(task_span_enter);
        std::mem::drop(task_span);

        let local_forge_index = self.load_index().await?;

        let (mut forge_index, _forge_index_hash) = local_forge_index.unwrap_or_default();

        let task_span = info_span!("process_forge_index_versions");
        task_span.pb_set_style(&super::progress_bar());
        task_span.pb_set_message("Processing Forge Maven ...");
        task_span.pb_set_length(forge_index.versions.len().try_into().unwrap_or_default());

        let task_span_enter = task_span.enter();

        // update recommendations for local versions, collect local versions
        let local_index_versions =
            HashSet::<(String, String)>::from_iter(forge_index.versions.iter_mut().map(
                |(long_version, forge_version)| {
                    let is_recommended = recommended_set.contains(&forge_version.version);
                    forge_version.recommended = Some(is_recommended);

                    if is_recommended {
                        forge_index
                            .by_mc_version
                            .get_mut(&forge_version.mc_version)
                            .unwrap_or_else(|| {
                                panic!(
                                    "Missing forge info for minecraft version {}",
                                    &forge_version.mc_version
                                )
                            })
                            .recommended = Some(long_version.clone());
                    }

                    task_span.pb_inc(1);
                    (forge_version.mc_version.clone(), long_version.clone())
                },
            ));

        let pending_forge_version_pairs = if !local_index_versions.is_empty() {
            let diff = remote_forge_version_pairs
                .difference(&local_index_versions)
                .cloned()
                .collect::<Vec<_>>();
            if !diff.is_empty() {
                info!(
                    "Missing local forge versions: {:?}",
                    diff.iter().map(|(_, lv)| lv).collect::<Vec<_>>()
                );
            }
            diff
        } else {
            info!("Local forge metadata does not exist, fetching all versions");
            remote_forge_version_pairs.into_iter().collect::<Vec<_>>()
        };

        std::mem::drop(task_span_enter);
        std::mem::drop(task_span);

        let task_span = info_span!("download_forge_versions");
        task_span.pb_set_style(&super::progress_bar());
        task_span.pb_set_message("Processing Forge Versions ...");
        task_span.pb_set_length(
            pending_forge_version_pairs
                .len()
                .try_into()
                .unwrap_or_default(),
        );

        let task_span_enter = task_span.enter();

        let tasks = stream::iter(pending_forge_version_pairs)
            .map(|(mc_version, long_version)| {
                let version_expression = regex::Regex::new(
                    "^(?P<mc>[0-9a-zA-Z_\\.]+)-(?P<ver>[0-9\\.]+\\.(?P<build>[0-9]+))(-(?P<branch>[a-zA-Z0-9\\.]+))?$"
                ).expect("Version regex must compile");
                let ls = self.clone();
                let recommended = recommended_set.clone();
                let spn = task_span.clone();
                tokio::spawn(async move {
                    let res = {
                        match version_expression.captures(&long_version) {
                            None => Err(eyre!("Forge long version '{0}' does not parse", long_version)),

                            Some(captures) => {
                                if captures.name("mc").is_none() {
                                    Err(eyre!("Forge long version '{0} not for a know minecraft version", long_version))
                                } else {
                                    ls.process_forge_version(
                                        &recommended,
                                        &mc_version,
                                        &long_version,
                                        captures.name("build").expect("Missing Forge build number").as_str().parse::<i32>()
                                            .wrap_err_with(|| format!("Failure parsing int build number for Forge version `{}`", long_version))?,
                                        captures.name("ver").expect("Missing Forge version").as_str(),
                                        captures.name("branch").map(|b| b.as_str().to_string()),
                                    )
                                    .await
                                }
                            }
                        }
                    };
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

        let (forge_versions, errors) = crate::utils::process_results(results);
        if !errors.is_empty() {
            return Err(MetadataError::BulkProcessingError(
                String::from("Updating Forge Metadata"),
                errors,
            )
            .into());
        }

        let task_span = info_span!("update_forge_index");
        task_span.pb_set_style(&super::progress_bar());
        task_span.pb_set_message("Updating Forge Version Index ...");
        task_span.pb_set_length(forge_versions.len().try_into().unwrap_or_default());

        let task_span_enter = task_span.enter();

        for forge_version in forge_versions {
            let mc_version = forge_version.mc_version.clone();
            let long_version = forge_version.long_version.clone();
            forge_index
                .versions
                .insert(forge_version.long_version.clone(), forge_version.clone());
            if !forge_index.by_mc_version.contains_key(&mc_version) {
                forge_index
                    .by_mc_version
                    .insert(mc_version.clone(), ForgeMCVersionInfo::default());
            }
            forge_index
                .by_mc_version
                .get_mut(&mc_version)
                .unwrap_or_else(|| {
                    panic!("Missing forge info for minecraft version {}", &mc_version)
                })
                .versions
                .push(long_version.clone());

            // NOTE: we add this later after the fact. The forge promotions file lies about these.
            // if let Some(true) = forge_version.latest {
            //     forge_index
            //         .by_mc_version
            //         .get_mut(&mc_version)
            //         .unwrap_or_else(|| {
            //             panic!("Missing forge info for minecraft version {}", &mc_version)
            //         })
            //         .latest = Some(long_version.clone());
            // }

            if let Some(true) = forge_version.recommended {
                forge_index
                    .by_mc_version
                    .get_mut(&mc_version)
                    .unwrap_or_else(|| {
                        panic!("Missing forge info for minecraft version {}", &mc_version)
                    })
                    .recommended = Some(long_version.clone());
            }
            task_span.pb_inc(1);
        }

        std::mem::drop(task_span_enter);
        std::mem::drop(task_span);

        info!("Post-processing forge promotions and adding missing 'latest'");

        for (mc_version, info) in forge_index.by_mc_version.iter_mut() {
            let latest_version = info
                .versions
                .last()
                .ok_or_else(|| eyre!("No forge versions for minecraft version {}", mc_version))?;
            info.latest = Some(latest_version.to_string());
            info!("Added {} as latest for {}", latest_version, mc_version)
        }

        info!("Storing forge index");
        self.store_maven_metadata(&maven_metadata).await?;
        self.store_forge_promotions(&promotions_metadata).await?;
        self.store_index(&forge_index).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn update_forge_installer_metadata(&self) -> Result<()> {
        let static_dir = std::path::Path::new(&self.config.static_directory);
        let forge_static_dir = static_dir.join("forge");
        if !forge_static_dir.is_dir() {
            info!(
                "Forge static metadata directory at {} does not exist, creating it",
                &forge_static_dir.to_string_lossy()
            );
            std::fs::create_dir_all(&forge_static_dir).wrap_err_with(|| {
                format!(
                    "Failed to create forge static dir {}",
                    &forge_static_dir.to_string_lossy()
                )
            })?;
        }
        let legacy_info_path = forge_static_dir.join("forge-legacyinfo.json");
        let aquire_legacy_info = !legacy_info_path.is_file();

        let mut legacy_info_list = ForgeLegacyInfoList::default();

        info!("Grabbing forge installers and storing installer profiles...");

        let Some((derived_index, derived_index_hash)) = self.load_index().await? else {
            return Err(eyre!("local forge index missing"));
        };

        if let Some(last_index) = self.load_index_entry().await? {
            // check if we even need to regenerate
            if last_index.hash == derived_index_hash {
                info!("Forge index up to date. Not regenerating.");
                return Ok(());
            } else {
                info!("Forge index hash did not match, regenerating...")
            }
        }

        let task_span = info_span!("download_forge_jars");
        task_span.pb_set_style(&super::progress_bar());
        task_span.pb_set_message("Processing Forge Installers ...");
        task_span.pb_set_length(derived_index.versions.len().try_into().unwrap_or_default());

        let task_span_enter = task_span.enter();

        // get the installer jars - if needed - and get the installer profiles out of them
        let tasks = stream::iter(derived_index.versions)
            .filter_map(|(key, entry)| async move {
                info!("Updating Forge {}", &key);
                let version = ForgeProcessedVersion::new(&entry);

                if version.url().is_none() {
                    warn!("Skipping forge build {} with no valid files", &entry.build);
                    return None;
                }

                if BAD_FORGE_VERSIONS.contains(&version.long_version.as_str()) {
                    info!("Skipping bad forge version {}", &version.long_version);
                    return None;
                }

                Some(version)
            })
            .map(|version| {
                let v = version.clone();
                let ls = self.clone();
                let ali = aquire_legacy_info;
                let spn = task_span.clone();
                tokio::spawn(async move {
                    let res = ls.process_forge_installer(&v, ali).await.wrap_err_with(|| {
                        format!(
                            "Failed to process forge installer jar for '{}'",
                            &v.long_version
                        )
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

        let legacy_version_infos = process_results_ok(results);

        for (long_version, version_info) in legacy_version_infos.into_iter().flatten() {
            legacy_info_list.number.insert(long_version, version_info);
        }

        // only write legacy info if it's missing
        if !legacy_info_path.is_file() {
            let legacy_info_json = serde_json::to_string_pretty(&legacy_info_list)?;
            std::fs::write(&legacy_info_path, legacy_info_json).wrap_err_with(|| {
                format!(
                    "Failure writing to file {}",
                    &legacy_info_path.to_string_lossy()
                )
            })?;
        }

        // update our index
        let last_index = MetaIndexEntry {
            update_time: chrono::Utc::now(),
            hash: derived_index_hash,
        };

        self.store_index_entry(&last_index).await?;

        Ok(())
    }

    pub fn meta_path(&self) -> ResourcePath {
        ResourcePath::new(["forge"])
    }

    pub fn manifests_dir(&self) -> ResourcePath {
        self.meta_path().join("files_manifests")
    }

    #[allow(dead_code)]
    pub async fn load_maven_metadata(&self) -> Result<Option<ForgeMavenMetadata>> {
        self.storage
            .fetch_record(self.meta_path(), "maven-metadata")
            .await
    }

    #[instrument(skip_all)]
    pub async fn store_maven_metadata(&self, metadata: &ForgeMavenMetadata) -> Result<()> {
        self.storage
            .store_record(self.meta_path(), "maven-metadata", metadata)
            .await
    }

    #[allow(dead_code)]
    pub async fn load_forge_promotions(&self) -> Result<Option<ForgeMavenPromotions>> {
        self.storage
            .fetch_record(self.meta_path(), "promotions_slim")
            .await
    }

    #[instrument(skip_all)]
    pub async fn store_forge_promotions(&self, promotions: &ForgeMavenPromotions) -> Result<()> {
        self.storage
            .store_record(self.meta_path(), "promotions_slim", promotions)
            .await
    }

    #[instrument(skip_all)]
    pub async fn load_index(&self) -> Result<Option<(DerivedForgeIndex, String)>> {
        self.storage
            .fetch_record::<DerivedForgeIndex>(self.meta_path(), "derived_index")
            .await
            .map(|index| {
                index.map(|index| {
                    info!("Computing hash of forge index");
                    let hash = crate::utils::hash(
                        serde_json::to_string(&index).expect("record loaded from json can't json?"),
                        crate::utils::HashAlgo::Sha256,
                    );
                    (index, hash)
                })
            })
    }

    #[instrument(skip_all)]
    pub async fn store_index(&self, index: &DerivedForgeIndex) -> Result<()> {
        self.storage
            .store_record(self.meta_path(), "derived_index", index)
            .await
    }

    pub async fn load_index_entry(&self) -> Result<Option<MetaIndexEntry>> {
        self.storage
            .fetch_record(self.meta_path(), "derived_index.last_index")
            .await
    }

    pub async fn store_index_entry(&self, index_entry: &MetaIndexEntry) -> Result<()> {
        self.storage
            .store_record(self.meta_path(), "derived_index.last_index", index_entry)
            .await
    }

    pub async fn load_files_manifest(
        &self,
        version_name: &str,
    ) -> Result<Option<ForgeVersionMeta>> {
        self.storage
            .fetch_record(self.manifests_dir(), version_name)
            .await
    }

    pub async fn store_files_manifest(
        &self,
        version_name: &str,
        manifest: &ForgeVersionMeta,
    ) -> Result<()> {
        self.storage
            .store_record(self.manifests_dir(), version_name, manifest)
            .await
    }

    pub fn installer_manifests_path(&self) -> ResourcePath {
        self.meta_path().join("installer_manifests")
    }

    pub async fn load_installer_manifest(
        &self,
        version_name: &str,
    ) -> Result<Option<ForgeInstallerProfile>> {
        self.storage
            .fetch_record(self.installer_manifests_path(), version_name)
            .await
    }

    pub async fn store_installer_manifest(
        &self,
        version_name: &str,
        manifest: &ForgeInstallerProfile,
    ) -> Result<()> {
        self.storage
            .store_record(self.installer_manifests_path(), version_name, manifest)
            .await
    }

    pub fn version_manifests_path(&self) -> ResourcePath {
        self.meta_path().join("version_manifests")
    }

    #[allow(dead_code)]
    pub async fn load_mojang_version(&self, version_name: &str) -> Result<Option<MojangVersion>> {
        self.storage
            .fetch_record(self.version_manifests_path(), version_name)
            .await
    }

    pub async fn store_mojang_version(
        &self,
        version_name: &str,
        version: &MojangVersion,
    ) -> Result<()> {
        self.storage
            .store_record(self.version_manifests_path(), version_name, version)
            .await
    }

    pub fn installer_info_path(&self) -> ResourcePath {
        self.meta_path().join("installer_info")
    }

    pub async fn load_installer_info(&self, version_name: &str) -> Result<Option<InstallerInfo>> {
        self.storage
            .fetch_record(self.installer_info_path(), version_name)
            .await
    }

    pub async fn store_installer_info(
        &self,
        version_name: &str,
        installer_info: &InstallerInfo,
    ) -> Result<()> {
        self.storage
            .store_record(self.installer_info_path(), version_name, installer_info)
            .await
    }

    async fn process_forge_version(
        &self,
        recommended_set: &HashSet<String>,
        mc_version: &str,
        long_version: &str,
        build: i32,
        version: &str,
        branch: Option<String>,
    ) -> Result<ForgeEntry> {
        let files = self.get_single_forge_files_manifest(long_version).await?;

        let is_recommended = recommended_set.contains(version);

        let entry = ForgeEntry {
            long_version: long_version.to_string(),
            mc_version: mc_version.to_string(),
            version: version.to_string(),
            build,
            branch,
            latest: None, // NOTE: we add this later after the fact. The forge promotions file lies about these.
            recommended: Some(is_recommended),
            files: Some(files),
        };

        Ok(entry)
    }

    #[instrument(skip(self))]
    async fn get_single_forge_files_manifest(
        &self,
        long_version: &str,
    ) -> Result<BTreeMap<String, ForgeFile>> {
        let files_manifest = self.load_files_manifest(long_version).await?;
        let files_metadata = if let Some(files_manifest) = files_manifest {
            info!("Forge manifest for {long_version} stored locally");
            files_manifest
        } else {
            info!("Getting Forge manifest for {long_version}");

            let file_url = format!(
                "https://files.minecraftforge.net/net/minecraftforge/forge/{}/meta.json",
                &long_version
            );
            let remote_manifest = self
                .load_single_forge_files_manifest(&file_url)
                .await
                .wrap_err_with(|| format!("Failure downloading {}", &file_url))?;
            self.store_files_manifest(long_version, &remote_manifest)
                .await?;
            remote_manifest
        };

        let mut ret_map: BTreeMap<String, ForgeFile> = BTreeMap::new();

        let re_w = regex::Regex::new("\\W").unwrap();

        for (classifier, extension_obj) in &files_metadata.classifiers {
            let mut count = 0;

            if let Some(extension_obj) = extension_obj {
                for (extension, hash_type) in extension_obj {
                    if let Some(hash_type) = hash_type {
                        let processed_hash = re_w.replace_all(hash_type, "");
                        if processed_hash.len() == 32 {
                            let file_obj = ForgeFile {
                                classifier: classifier.as_str().to_owned(),
                                hash: processed_hash.to_string(),
                                extension: extension.as_str().to_owned(),
                            };
                            if count == 0 {
                                ret_map.insert(classifier.as_str().to_string(), file_obj);
                                count += 1;
                            } else {
                                return Err(eyre!(
                                    "{}: Multiple objects detected for classifier {}: {:?}",
                                    long_version,
                                    extension.as_str(),
                                    &extension_obj
                                ));
                            }
                        } else {
                            debug!(
                                "{}: Skipping invalid hash for extension {}: {:?}",
                                &long_version,
                                extension.as_str(),
                                &extension_obj
                            )
                        }
                    } else {
                        debug!(
                            "{}: Skipping missing hash for extension {}",
                            &long_version,
                            extension.as_str()
                        );
                    }
                }
            }
        }
        Ok(ret_map)
    }

    #[instrument(skip(self, version), fields(forge_version = version.long_version ))]
    async fn process_forge_installer(
        &self,
        version: &ForgeProcessedVersion,
        aquire_legacy_info: bool,
    ) -> Result<Option<(String, ForgeLegacyInfo)>> {
        let tmp_dir = TempDir::new("mcmeta_forge_installer_jar")?;
        let jar_path = tmp_dir.path().join(&version.filename().ok_or_else(|| {
            eyre!(
                "Missing forge filename for version {}",
                &version.long_version
            )
        })?);

        if version.uses_installer() {
            let installer_info = self.load_installer_info(&version.long_version).await?;
            let profile = self.load_installer_manifest(&version.long_version).await?;

            let installer_refresh_required = profile.is_none()
                || installer_info.is_none()
                || installer_info.as_ref().is_some_and(|info| {
                    let version_hash = version.hash();
                    if !version.hash_match(&info.md5hash) {
                        warn!(
                            "Hash mismatch for Forge Installer {} : {:?} != {:?}",
                            &version.long_version, &info.md5hash, &version_hash
                        );
                        true
                    } else {
                        false
                    }
                });

            if installer_refresh_required {
                // grab the installer if it's not there
                if !jar_path.is_file() {
                    info!("Downloading forge jar from {}", &version.url().unwrap());
                    upstream::download_binary_file(&self.client, &jar_path, &version.url().unwrap())
                        .await
                        .wrap_err_with(|| {
                            format!("Failure downloading {}", &version.url().unwrap())
                        })?
                }
            }

            info!("Processing forge jar from {}", &version.url().unwrap());
            if profile.is_none() {
                use std::io::Read;

                let mut jar =
                    zip::ZipArchive::new(std::fs::File::open(&jar_path).wrap_err_with(|| {
                        format!("Failure opening {}", &jar_path.to_string_lossy())
                    })?)
                    .wrap_err_with(|| {
                        format!(
                            "Failure reading Jar archive {}",
                            &jar_path.to_string_lossy()
                        )
                    })?;

                if let Some(mojang_version) = {
                    // version.json
                    if let Ok(mut version_zip_entry) = jar.by_name("version.json") {
                        let mut version_data = String::new();
                        version_zip_entry
                            .read_to_string(&mut version_data)
                            .wrap_err_with(|| {
                                format!(
                                    "Failure reading 'version.json' from {}",
                                    &jar_path.to_string_lossy()
                                )
                            })?;

                        let mojang_version: MojangVersion =
                            crate::utils::deserialize_json_with_error_path(&version_data)
                                .wrap_err_with(|| {
                                    format!(
                                        "Failure reading json from 'version.json' in {}",
                                        &jar_path.to_string_lossy()
                                    )
                                })?;
                        Some(mojang_version)
                    } else {
                        None
                    }
                } {
                    self.store_mojang_version(&version.long_version, &mojang_version)
                        .await?;
                }

                let forge_profile = {
                    //install_profile.json
                    let mut profile_zip_entry =
                        jar.by_name("install_profile.json").wrap_err_with(|| {
                            format!(
                                "{} is missing install_profile.json",
                                &jar_path.to_string_lossy()
                            )
                        })?;
                    let mut install_profile_data = String::new();
                    profile_zip_entry
                        .read_to_string(&mut install_profile_data)
                        .wrap_err_with(|| {
                            format!(
                                "Failure reading 'install_profile.json' from {}",
                                &jar_path.to_string_lossy()
                            )
                        })?;

                    crate::utils::deserialize_json_with_error_path::<ForgeInstallerProfile>(
                        &install_profile_data,
                    )
                };

                if let Ok(forge_profile) = forge_profile {
                    self.store_installer_manifest(&version.long_version, &forge_profile)
                        .await?;
                } else if version.is_supported() {
                    return Err(forge_profile.unwrap_err()).wrap_err_with(|| {
                        format!(
                            "Failure reading json from 'install_profile.json' in {}",
                            &jar_path.to_string_lossy()
                        )
                    });
                } else {
                    debug!(
                        "Forge Version {} is not supported and won't be generated later.",
                        &version.long_version
                    )
                }
            }

            if installer_info.is_none()
                || installer_info
                    .as_ref()
                    .is_some_and(|info| !version.hash_match(&info.md5hash))
            {
                let installer_info = InstallerInfo {
                    sha1hash: Some(filehash(&jar_path, HashAlgo::Sha1)?),
                    sha256hash: Some(filehash(&jar_path, HashAlgo::Sha256)?),
                    md5hash: Some(filehash(&jar_path, HashAlgo::Md5)?),
                    size: Some(jar_path.metadata()?.len()),
                };

                self.store_installer_info(&version.long_version, &installer_info)
                    .await?;
            }
            Ok(None)
        } else {
            // ignore the two versions without install manifests and jar mod class files
            // TODO: fix those versions?

            if version.mc_version_sane == "1.6.1" {
                return Ok(None);
            }

            // only gather legacy info if it's missing
            if aquire_legacy_info {
                if !jar_path.is_file() {
                    info!("Downloading forge jar from {}", &version.url().unwrap());
                    upstream::download_binary_file(&self.client, &jar_path, &version.url().unwrap())
                        .await
                        .wrap_err_with(|| {
                            format!("Failure downloading {}", &version.url().unwrap())
                        })?
                }

                // find the latest timestamp in the zip file
                let mut time_stamp = chrono::NaiveDateTime::UNIX_EPOCH;

                {
                    // context drop to close file
                    let mut jar =
                        zip::ZipArchive::new(std::fs::File::open(&jar_path).wrap_err_with(
                            || format!("Failure opening {}", &jar_path.to_string_lossy()),
                        )?)
                        .wrap_err_with(|| {
                            format!(
                                "Failure reading Jar archive {}",
                                &jar_path.to_string_lossy()
                            )
                        })?;

                    for i in 0..jar.len() {
                        let file = jar.by_index(i).wrap_err_with(|| {
                            format!(
                                "Failure reading Jar archive {} `index:{}`",
                                &jar_path.to_string_lossy(),
                                i
                            )
                        })?;
                        let time_stamp_new = file
                            .last_modified()
                            .map(TryInto::<chrono::NaiveDateTime>::try_into)
                            .transpose()?
                            .ok_or_else(|| {
                                eyre!(
                                    "Failure reading Jar archive {} `index:{}` last modified time",
                                    &jar_path.to_string_lossy(),
                                    i
                                )
                            })?;
                        if time_stamp_new > time_stamp {
                            time_stamp = time_stamp_new;
                        }
                    }
                }

                let legacy_info = ForgeLegacyInfo {
                    release_time: Some(chrono::Utc.from_utc_datetime(&time_stamp)),
                    sha1: Some(filehash(&jar_path, HashAlgo::Sha1)?),
                    sha256: Some(filehash(&jar_path, HashAlgo::Sha256)?),
                    size: Some(jar_path.metadata()?.len()),
                };

                return Ok(Some((version.long_version.clone(), legacy_info)));
                // legacy_info_list.number.insert(key, legacy_info);
            }
            Ok(None)
        }
    }
}

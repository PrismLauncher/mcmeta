use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use futures::{stream, StreamExt};
use std::collections::{BTreeMap, HashSet};
use tracing::{debug, info, warn};

use crate::{
    download,
    storage::{StorageFormat, UpstreamMetadataUpdater},
    utils::{filehash, hash, process_results, process_results_ok, HashAlgo},
};
use libmcmeta::models::forge::{
    DerivedForgeIndex, ForgeEntry, ForgeFile, ForgeInstallerProfile, ForgeLegacyInfo,
    ForgeLegacyInfoList, ForgeMCVersionInfo, ForgeMavenMetadata, ForgeMavenPromotions,
    ForgeProcessedVersion, ForgeVersionMeta, InstallerInfo,
};
use libmcmeta::models::mojang::MojangVersion;
use libmcmeta::models::MetaMcIndexEntry;

lazy_static! {
    pub static ref BAD_FORGE_VERSIONS: Vec<&'static str> = vec!["1.12.2-14.23.5.2851"];
}

#[derive(Clone)]
pub struct ForgeDataStorage {
    storage_format: Arc<StorageFormat>,
}

impl ForgeDataStorage {
    pub fn meta_dir(&self) -> Result<std::path::PathBuf> {
        match *self.storage_format {
            StorageFormat::Json {
                ref meta_directory,
                generated_directory: _,
            } => {
                let metadata_dir = std::path::Path::new(&meta_directory);
                let forge_meta_dir = metadata_dir.join("forge");

                if !forge_meta_dir.is_dir() {
                    info!(
                        "Forge metadata directory at {} does not exist, creating it",
                        forge_meta_dir.display()
                    );
                    std::fs::create_dir_all(&forge_meta_dir)?;
                }
                Ok(forge_meta_dir)
            }
            StorageFormat::Database => Err(anyhow!("Wrong storage format")),
        }
    }

    pub fn manifests_dir(&self) -> Result<std::path::PathBuf> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let forge_file_manifest_path = self.meta_dir()?.join("files_manifests");

                if !forge_file_manifest_path.is_dir() {
                    info!(
                        "Forge files manifests directory at {} does not exist, creating it",
                        forge_file_manifest_path.display()
                    );
                    std::fs::create_dir_all(&forge_file_manifest_path)?;
                }
                Ok(forge_file_manifest_path)
            }
            StorageFormat::Database => Err(anyhow!("Wrong storage format")),
        }
    }

    pub fn load_maven_metadata(&self) -> Result<Option<ForgeMavenMetadata>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let maven_metadata_file = self.meta_dir()?.join("maven-metadata.json");
                if maven_metadata_file.is_file() {
                    let metadata = serde_json::from_str::<ForgeMavenMetadata>(
                        &std::fs::read_to_string(&maven_metadata_file).with_context(|| {
                            format!(
                                "Failure reading from file {}",
                                &maven_metadata_file.to_string_lossy()
                            )
                        })?,
                    )?;
                    Ok(Some(metadata))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn store_maven_metadata(&self, metadata: &ForgeMavenMetadata) -> Result<()> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let maven_metadata_file = self.meta_dir()?.join("maven-metadata.json");
                let maven_metadata_json = serde_json::to_string_pretty(&metadata)?;
                std::fs::write(&maven_metadata_file, maven_metadata_json).with_context(|| {
                    format!(
                        "Failure writing to file {}",
                        &maven_metadata_file.to_string_lossy()
                    )
                })?;
                Ok(())
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn load_forge_promotions(&self) -> Result<Option<ForgeMavenPromotions>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let promotions_metadata_file = self.meta_dir()?.join("promotions_slim.json");
                if promotions_metadata_file.is_file() {
                    let promotions = serde_json::from_str::<ForgeMavenPromotions>(
                        &std::fs::read_to_string(&promotions_metadata_file).with_context(|| {
                            format!(
                                "Failure reading from file {}",
                                &promotions_metadata_file.to_string_lossy()
                            )
                        })?,
                    )?;
                    Ok(Some(promotions))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn store_forge_promotions(&self, promotions: &ForgeMavenPromotions) -> Result<()> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let promotions_metadata_file = self.meta_dir()?.join("promotions_slim.json");
                let promotions_metadata_json = serde_json::to_string_pretty(&promotions)?;
                std::fs::write(&promotions_metadata_file, promotions_metadata_json).with_context(
                    || {
                        format!(
                            "Failure writing to file {}",
                            &promotions_metadata_file.to_string_lossy()
                        )
                    },
                )?;

                Ok(())
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn load_index(&self) -> Result<Option<DerivedForgeIndex>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let derived_index_file = self.meta_dir()?.join("derived_index.json");
                if derived_index_file.is_file() {
                    let index = serde_json::from_str::<DerivedForgeIndex>(
                        &std::fs::read_to_string(&derived_index_file).with_context(|| {
                            format!(
                                "Failure reading from file {}",
                                &derived_index_file.to_string_lossy()
                            )
                        })?,
                    )?;
                    Ok(Some(index))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn store_index(&self, index: &DerivedForgeIndex) -> Result<()> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let local_derived_index_file = self.meta_dir()?.join("derived_index.json");
                let derived_index_json = serde_json::to_string_pretty(&index)?;
                std::fs::write(&local_derived_index_file, derived_index_json).with_context(
                    || {
                        format!(
                            "Failure writing to file {}",
                            &local_derived_index_file.to_string_lossy()
                        )
                    },
                )?;
                Ok(())
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn index_hash(&self) -> Result<Option<String>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let derived_index_file = self.meta_dir()?.join("derived_index.json");
                if derived_index_file.is_file() {
                    let contents =
                        &std::fs::read_to_string(&derived_index_file).with_context(|| {
                            format!(
                                "Failure reading from file {}",
                                &derived_index_file.to_string_lossy()
                            )
                        })?;
                    let hash_str = hash(contents, HashAlgo::Sha256)?;
                    Ok(Some(hash_str))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(), // use utils::hash insted of filehash
        }
    }

    pub fn load_index_entry(&self) -> Result<Option<MetaMcIndexEntry>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let last_index_path = self.meta_dir()?.join("derived_index.last_index.json");
                if last_index_path.is_file() {
                    Ok(Some(serde_json::from_str::<MetaMcIndexEntry>(
                        &std::fs::read_to_string(&last_index_path).with_context(|| {
                            format!("Failure opening {}", &last_index_path.to_string_lossy())
                        })?,
                    )?))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn store_index_entry(&self, index_entry: &MetaMcIndexEntry) -> Result<()> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let mut entry = index_entry.clone();
                let derived_index_file = self.meta_dir()?.join("derived_index.json");
                let last_index_path = self.meta_dir()?.join("derived_index.last_index.json");
                entry.path = derived_index_file.to_string_lossy().to_string();
                let last_index_json = serde_json::to_string_pretty(&entry)?;
                std::fs::write(&last_index_path, last_index_json).with_context(|| {
                    format!(
                        "Failure writing to file {}",
                        &last_index_path.to_string_lossy()
                    )
                })?
            }
            StorageFormat::Database => todo!(),
        }
        Ok(())
    }

    pub fn load_files_manifest(&self, version_name: &str) -> Result<Option<ForgeVersionMeta>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let files_manifest_file =
                    self.manifests_dir()?.join(format!("{}.json", version_name));
                if files_manifest_file.is_file() {
                    let files_manifest = serde_json::from_str::<ForgeVersionMeta>(
                        &std::fs::read_to_string(&files_manifest_file).with_context(|| {
                            format!(
                                "Failure reading file {}",
                                &files_manifest_file.to_string_lossy()
                            )
                        })?,
                    )?;
                    Ok(Some(files_manifest))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn store_files_manifest(
        &self,
        version_name: &str,
        manifest: &ForgeVersionMeta,
    ) -> Result<()> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let files_manifest_file =
                    self.manifests_dir()?.join(format!("{}.json", version_name));

                let files_metadata_json = serde_json::to_string_pretty(&manifest)?;
                std::fs::write(&files_manifest_file, files_metadata_json).with_context(|| {
                    format!(
                        "Failure writing to file {}",
                        &files_manifest_file.to_string_lossy()
                    )
                })?;
            }
            StorageFormat::Database => todo!(),
        }
        Ok(())
    }

    pub fn forge_jars_dir(&self) -> Result<std::path::PathBuf> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let jar_dir = self.meta_dir()?.join("jars");
                if !jar_dir.is_dir() {
                    info!(
                        "Forge jar directory at {} does not exist, creating it",
                        jar_dir.display()
                    );
                    std::fs::create_dir_all(&jar_dir)?;
                }
                Ok(jar_dir)
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn installer_manifests_dir(&self) -> Result<std::path::PathBuf> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let installer_manifests_dir = self.meta_dir()?.join("installer_manifests");
                if !installer_manifests_dir.is_dir() {
                    info!(
                        "Forge installer manifests directory at {} does not exist, creating it",
                        installer_manifests_dir.display()
                    );
                    std::fs::create_dir_all(&installer_manifests_dir)?;
                }
                Ok(installer_manifests_dir)
            }
            StorageFormat::Database => Err(anyhow!("Wrong storage format")),
        }
    }

    pub fn load_installer_manifest(
        &self,
        version_name: &str,
    ) -> Result<Option<ForgeInstallerProfile>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let installer_manifest_file = self
                    .installer_manifests_dir()?
                    .join(format!("{}.json", version_name));
                if installer_manifest_file.is_file() {
                    let installer_manifest = serde_json::from_str::<ForgeInstallerProfile>(
                        &std::fs::read_to_string(&installer_manifest_file).with_context(|| {
                            format!(
                                "Failure reading file {}",
                                &installer_manifest_file.to_string_lossy()
                            )
                        })?,
                    )?;
                    Ok(Some(installer_manifest))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn store_installer_manifest(
        &self,
        version_name: &str,
        manifest: &ForgeInstallerProfile,
    ) -> Result<()> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let installer_manifest_file = self
                    .installer_manifests_dir()?
                    .join(format!("{}.json", version_name));

                let installer_manifest_json = serde_json::to_string_pretty(&manifest)?;
                std::fs::write(&installer_manifest_file, installer_manifest_json).with_context(
                    || {
                        format!(
                            "Failure writing to file {}",
                            &installer_manifest_file.to_string_lossy()
                        )
                    },
                )?;
            }
            StorageFormat::Database => todo!(),
        }
        Ok(())
    }

    pub fn version_manifests_dir(&self) -> Result<std::path::PathBuf> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let version_manifests_dir = self.meta_dir()?.join("version_manifests");
                if !version_manifests_dir.is_dir() {
                    info!(
                        "Forge verison manifests directory at {} does not exist, creating it",
                        version_manifests_dir.display()
                    );
                    std::fs::create_dir_all(&version_manifests_dir)?;
                }
                Ok(version_manifests_dir)
            }
            StorageFormat::Database => Err(anyhow!("Wrong storage format")),
        }
    }

    pub fn load_mojang_version(&self, version_name: &str) -> Result<Option<MojangVersion>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let version_manifest_file = self
                    .version_manifests_dir()?
                    .join(format!("{}.json", version_name));
                if version_manifest_file.is_file() {
                    let version_manifest = serde_json::from_str::<MojangVersion>(
                        &std::fs::read_to_string(&version_manifest_file).with_context(|| {
                            format!(
                                "Failure reading file {}",
                                &version_manifest_file.to_string_lossy()
                            )
                        })?,
                    )?;
                    Ok(Some(version_manifest))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn store_mojang_version(&self, version_name: &str, version: &MojangVersion) -> Result<()> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let version_manifest_file = self
                    .installer_manifests_dir()?
                    .join(format!("{}.json", version_name));

                let version_manifest_json = serde_json::to_string_pretty(&version)?;
                std::fs::write(&version_manifest_file, version_manifest_json).with_context(
                    || {
                        format!(
                            "Failure writing to file {}",
                            &version_manifest_file.to_string_lossy()
                        )
                    },
                )?;
            }
            StorageFormat::Database => todo!(),
        }
        Ok(())
    }

    pub fn installer_info_dir(&self) -> Result<std::path::PathBuf> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let installer_info_dir = self.meta_dir()?.join("installer_info");
                if !installer_info_dir.is_dir() {
                    info!(
                        "Forge installer info directory at {} does not exist, creating it",
                        installer_info_dir.display()
                    );
                    std::fs::create_dir_all(&installer_info_dir)?;
                }
                Ok(installer_info_dir)
            }
            StorageFormat::Database => Err(anyhow!("Wrong storage format")),
        }
    }

    pub fn load_installer_info(&self, version_name: &str) -> Result<Option<InstallerInfo>> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let version_manifest_file = self
                    .version_manifests_dir()?
                    .join(format!("{}.json", version_name));
                if version_manifest_file.is_file() {
                    let version_manifest = serde_json::from_str::<InstallerInfo>(
                        &std::fs::read_to_string(&version_manifest_file).with_context(|| {
                            format!(
                                "Failure reading file {}",
                                &version_manifest_file.to_string_lossy()
                            )
                        })?,
                    )?;
                    Ok(Some(version_manifest))
                } else {
                    Ok(None)
                }
            }
            StorageFormat::Database => todo!(),
        }
    }

    pub fn store_installer_info(
        &self,
        version_name: &str,
        installer_info: &InstallerInfo,
    ) -> Result<()> {
        match *self.storage_format {
            StorageFormat::Json {
                meta_directory: _,
                generated_directory: _,
            } => {
                let installer_info_file = self
                    .installer_manifests_dir()?
                    .join(format!("{}.json", version_name));

                let installer_info_json = serde_json::to_string_pretty(&installer_info)?;
                std::fs::write(&installer_info_file, installer_info_json).with_context(|| {
                    format!(
                        "Failure writing to file {}",
                        &installer_info_file.to_string_lossy()
                    )
                })?;
            }
            StorageFormat::Database => todo!(),
        }
        Ok(())
    }
}

impl UpstreamMetadataUpdater {
    pub async fn initialize_forge_metadata(&self) -> Result<()> {
        info!("Checking for Forge metadata");
        self.update_forge_metadata()
            .await
            .with_context(|| "Failed to update Forge metadata.")?;

        self.update_forge_installer_metadata()
            .await
            .with_context(|| "Failed to update Forge legacy metadata.")?;
        Ok(())
    }

    pub async fn update_forge_metadata(&self) -> Result<()> {
        let local_storage = ForgeDataStorage {
            storage_format: self.storage_format.clone(),
        };

        let maven_metadata = download::forge::load_maven_metadata().await?;
        let promotions_metadata = download::forge::load_maven_promotions().await?;

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
        debug!("Processing Forge Promotions");

        for (promo_key, shortversion) in &promotions_metadata.promos {
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

        debug!("Processing Forge Versions");
        let remote_forge_version_pairs =
            HashSet::<(String, String)>::from_iter(maven_metadata.versions.iter().flat_map(
                |(mc_version, forge_version_list)| {
                    forge_version_list
                        .iter()
                        .map(|forge_version| (mc_version.clone(), forge_version.clone()))
                },
            ));

        let local_forge_index = local_storage.load_index()?;

        let mut process_all = false;
        let mut forge_index = if let Some(local_forge_index) = local_forge_index {
            process_all = local_forge_index.versions.is_empty();
            DerivedForgeIndex {
                versions: local_forge_index.versions.clone(),
                by_mc_version: local_forge_index.by_mc_version.clone(),
            }
        } else {
            DerivedForgeIndex::default()
        };

        // update recomendations for local versions
        for (long_version, forge_version) in forge_index.versions.iter_mut() {
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
        }

        let pending_forge_version_pairs = match local_storage.load_maven_metadata()? {
            Some(local_maven_metadata) if !process_all => {
                let local_forge_version_pairs = HashSet::<(String, String)>::from_iter(
                    local_maven_metadata.versions.iter().flat_map(
                        |(mc_version, forge_version_list)| {
                            forge_version_list
                                .iter()
                                .map(|forge_version| (mc_version.clone(), forge_version.clone()))
                        },
                    ),
                );
                let diff = remote_forge_version_pairs
                    .difference(&local_forge_version_pairs)
                    .cloned()
                    .collect::<Vec<_>>();
                if !diff.is_empty() {
                    info!(
                        "Missing local forge versions: {:?}",
                        diff.iter().map(|(_, lv)| lv).collect::<Vec<_>>()
                    );
                }
                diff
            }
            _ => {
                info!("Local forge metadata does not exist, fetching all versions");
                remote_forge_version_pairs.into_iter().collect::<Vec<_>>()
            }
        };
        let tasks = stream::iter(pending_forge_version_pairs)
            .map(|(mc_version, long_version)| {
                let version_expression = regex::Regex::new(
                    "^(?P<mc>[0-9a-zA-Z_\\.]+)-(?P<ver>[0-9\\.]+\\.(?P<build>[0-9]+))(-(?P<branch>[a-zA-Z0-9\\.]+))?$"
                ).expect("Version regex must compile");
                let ls = local_storage.clone();
                let recommended = recommended_set.clone();
                tokio::spawn(async move {
                    match version_expression.captures(&long_version) {
                        None => Err(anyhow!(
                            "Forge long version {} does not parse!",
                            long_version
                        )),

                        Some(captures) => {
                            if captures.name("mc").is_none() {
                                Err(anyhow!(
                                    "Forge long version {} not for a minecraft version?",
                                    long_version
                                ))
                            } else {
                                process_forge_version(
                                    &ls,
                                    &recommended,
                                    &mc_version,
                                    &long_version,
                                    captures.name("build").expect("Missing Forge build number").as_str().parse::<i32>()
                                        .with_context(|| format!("Failure parsing int build number for Forge version `{}`", long_version))?,
                                    captures.name("ver").expect("Missing Forge version").as_str(),
                                    captures.name("branch").map(|b| b.as_str().to_string()),
                                )
                                .await
                            }
                        }
                    }
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
        let forge_versions = process_results(results)?;

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
            //     new_index.by_mc_version[&mc_version].latest = Some(long_version.clone());
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
        }

        debug!("Post-processing forge promotions and adding missing 'latest'");

        for (mc_version, info) in forge_index.by_mc_version.iter_mut() {
            let latest_version = info.versions.last().unwrap_or_else(|| {
                panic!("No forge versions for minecraft version {}", mc_version)
            });
            info.latest = Some(latest_version.to_string());
            info!("Added {} as latest for {}", latest_version, mc_version)
        }

        debug!("Dumping forge index files");
        local_storage.store_maven_metadata(&maven_metadata)?;
        local_storage.store_forge_promotions(&promotions_metadata)?;
        local_storage.store_index(&forge_index)?;

        Ok(())
    }

    pub async fn update_forge_installer_metadata(&self) -> Result<()> {
        let local_storage = ForgeDataStorage {
            storage_format: self.storage_format.clone(),
        };

        let static_dir = std::path::Path::new(&self.metadata_cfg.static_directory);
        let forge_static_dir = static_dir.join("forge");
        if !forge_static_dir.is_dir() {
            info!(
                "Forge static metadata directory at {} does not exist, creating it",
                &forge_static_dir.to_string_lossy()
            );
            std::fs::create_dir_all(&forge_static_dir).with_context(|| {
                format!(
                    "Failed to create forge static dir {}",
                    &forge_static_dir.to_string_lossy()
                )
            })?;
        }
        let legacy_info_path = forge_static_dir.join("forge-legacyinfo.json");
        let aquire_legacy_info = !legacy_info_path.is_file();

        let mut legacy_info_list = ForgeLegacyInfoList::default();

        debug!("Grabbing forge installers and dumping installer profiles...");

        let derived_index = local_storage
            .load_index()?
            .ok_or(anyhow!("local forge index missing"))?;

        let derived_index_hash = local_storage
            .index_hash()?
            .ok_or(anyhow!("local forge index missing"))?;

        if let Some(last_index) = local_storage.load_index_entry()? {
            // check if we even need to regenerate
            if last_index.hash == derived_index_hash {
                info!("Forge index up to date. Not regenerating.");
                return Ok(());
            } else {
                info!("Forge index hash did not match, regenerating...")
            }
        }

        // get the installer jars - if needed - and get the installer profiles out of them
        let tasks = stream::iter(derived_index.versions)
            .filter_map(|(key, entry)| async move {
                info!("Updating Forge {}", &key);
                let version = ForgeProcessedVersion::new(&entry);

                if version.url().is_none() {
                    debug!("Skipping forge build {} with no valid files", &entry.build);
                    return None;
                }

                if BAD_FORGE_VERSIONS.contains(&version.long_version.as_str()) {
                    debug!("Skipping bad forge version {}", &version.long_version);
                    return None;
                }

                Some(version)
            })
            .map(|version| {
                let ls = local_storage.clone();
                tokio::spawn(async move {
                    process_forge_installer(&ls, &version, aquire_legacy_info).await
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

        let legacy_version_infos = process_results_ok(results);

        for (long_version, version_info) in legacy_version_infos.into_iter().flatten() {
            legacy_info_list.number.insert(long_version, version_info);
        }

        // only write legacy info if it's missing
        if !legacy_info_path.is_file() {
            let legacy_info_json = serde_json::to_string_pretty(&legacy_info_list)?;
            std::fs::write(&legacy_info_path, legacy_info_json).with_context(|| {
                format!(
                    "Failure writing to file {}",
                    &legacy_info_path.to_string_lossy()
                )
            })?;
        }

        // update our index
        let last_index = MetaMcIndexEntry {
            update_time: time::OffsetDateTime::now_utc(),
            path: "".to_owned(),
            hash: derived_index_hash,
        };

        local_storage.store_index_entry(&last_index)?;

        Ok(())
    }
}

async fn process_forge_version(
    local_storage: &ForgeDataStorage,
    recommended_set: &HashSet<String>,
    mc_version: &str,
    long_version: &str,
    build: i32,
    version: &str,
    branch: Option<String>,
) -> Result<ForgeEntry> {
    let files = get_single_forge_files_manifest(&local_storage, long_version).await?;

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

async fn get_single_forge_files_manifest(
    local_storage: &ForgeDataStorage,
    long_version: &str,
) -> Result<BTreeMap<String, ForgeFile>> {
    let files_manifest = local_storage.load_files_manifest(&long_version)?;
    let files_metadata = if let Some(files_manifest) = files_manifest {
        info!("Forge manifest for {long_version} stored localy");
        files_manifest
    } else {
        info!("Getting Forge manifest for {long_version}");

        let file_url = format!(
            "https://files.minecraftforge.net/net/minecraftforge/forge/{}/meta.json",
            &long_version
        );
        let remote_manifest = download::forge::load_single_forge_files_manifest(&file_url)
            .await
            .with_context(|| format!("Failure downloading {}", &file_url))?;
        local_storage.store_files_manifest(&long_version, &remote_manifest)?;
        remote_manifest
    };

    let mut ret_map: BTreeMap<String, ForgeFile> = BTreeMap::new();

    for (classifier, extension_obj) in &files_metadata.classifiers {
        let mut count = 0;

        if let Some(extension_obj) = extension_obj {
            for (extension, hash_type) in extension_obj {
                if let Some(hash_type) = hash_type {
                    let re = regex::Regex::new("\\W").unwrap();
                    let processed_hash = re.replace_all(hash_type, "");
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
                            return Err(anyhow!(
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

async fn process_forge_installer(
    local_storage: &ForgeDataStorage,
    version: &ForgeProcessedVersion,
    aquire_legacy_info: bool,
) -> Result<Option<(String, ForgeLegacyInfo)>> {
    let jar_path = local_storage
        .forge_jars_dir()?
        .join(&version.filename().expect("Missing forge filename"));

    if version.uses_installer() {
        let installer_info = local_storage.load_installer_info(&version.long_version)?;
        let profile = local_storage.load_installer_manifest(&version.long_version)?;

        let installer_refresh_required = profile.is_none() || installer_info.is_none();

        if installer_refresh_required {
            // grab the installer if it's not there
            if !jar_path.is_file() {
                debug!("Downloading forge jar from {}", &version.url().unwrap());
                download::download_binary_file(&jar_path, &version.url().unwrap())
                    .await
                    .with_context(|| format!("Failure downloading {}", &version.url().unwrap()))?
            }
        }

        debug!("Processing forge jar from {}", &version.url().unwrap());
        if profile.is_none() {
            use std::io::Read;

            let mut jar = zip::ZipArchive::new(
                std::fs::File::open(&jar_path)
                    .with_context(|| format!("Failure opening {}", &jar_path.to_string_lossy()))?,
            )
            .with_context(|| {
                format!(
                    "Failure reading Jar archive {}",
                    &jar_path.to_string_lossy()
                )
            })?;

            {
                // version.json
                if let Ok(mut version_zip_entry) = jar.by_name("version.json") {
                    let mut version_data = String::new();
                    version_zip_entry
                        .read_to_string(&mut version_data)
                        .with_context(|| {
                            format!(
                                "Failure reading 'version.json' from {}",
                                &jar_path.to_string_lossy()
                            )
                        })?;

                    let mojang_version: MojangVersion = serde_json::from_str(&version_data)
                        .with_context(|| {
                            format!(
                                "Failure reading json from 'version.json' in {}",
                                &jar_path.to_string_lossy()
                            )
                        })?;

                    local_storage.store_mojang_version(&version.long_version, &mojang_version)?;
                }
            }

            {
                //install_profile.json
                let mut profile_zip_entry =
                    jar.by_name("install_profile.json").with_context(|| {
                        format!(
                            "{} is missing install_profile.json",
                            &jar_path.to_string_lossy()
                        )
                    })?;
                let mut install_profile_data = String::new();
                profile_zip_entry
                    .read_to_string(&mut install_profile_data)
                    .with_context(|| {
                        format!(
                            "Failure reading 'install_profile.json' from {}",
                            &jar_path.to_string_lossy()
                        )
                    })?;

                let forge_profile =
                    serde_json::from_str::<ForgeInstallerProfile>(&install_profile_data);
                if let Ok(forge_profile) = forge_profile {
                    local_storage
                        .store_installer_manifest(&version.long_version, &forge_profile)?;
                } else if version.is_supported() {
                    return Err(forge_profile.unwrap_err()).with_context(|| {
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
        }

        if installer_info.is_none() {
            let installer_info = InstallerInfo {
                sha1hash: Some(filehash(&jar_path, HashAlgo::Sha1)?),
                sha256hash: Some(filehash(&jar_path, HashAlgo::Sha256)?),
                size: Some(jar_path.metadata()?.len()),
            };

            local_storage.store_installer_info(&version.long_version, &installer_info)?;
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
                debug!("Downloading forge jar from {}", &version.url().unwrap());
                download::download_binary_file(&jar_path, &version.url().unwrap())
                    .await
                    .with_context(|| format!("Failure downloading {}", &version.url().unwrap()))?
            }

            // find the latest timestamp in the zip file
            let mut time_stamp = time::OffsetDateTime::UNIX_EPOCH;

            {
                // context drop to close file
                let mut jar =
                    zip::ZipArchive::new(std::fs::File::open(&jar_path).with_context(|| {
                        format!("Failure opening {}", &jar_path.to_string_lossy())
                    })?)
                    .with_context(|| {
                        format!(
                            "Failure reading Jar archive {}",
                            &jar_path.to_string_lossy()
                        )
                    })?;

                for i in 0..jar.len() {
                    let file = jar.by_index(i).with_context(|| {
                        format!(
                            "Failure reading Jar archive {} `index:{}`",
                            &jar_path.to_string_lossy(),
                            i
                        )
                    })?;
                    let time_stamp_new = file.last_modified().to_time().with_context(|| {
                        format!(
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
                release_time: Some(time_stamp),
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

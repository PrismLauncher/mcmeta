use anyhow::{anyhow, Context, Result};
use futures::{stream, StreamExt};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use tracing::{debug, error, info, warn};

use crate::{
    app_config::MetadataConfig,
    download,
    storage::StorageFormat,
    utils::{filehash, HashAlgo},
};
use libmcmeta::models::forge::{
    DerivedForgeIndex, ForgeEntry, ForgeFile, ForgeInstallerProfile, ForgeInstallerProfileV2,
    ForgeLegacyInfo, ForgeLegacyInfoList, ForgeMCVersionInfo, ForgeProcessedVersion,
    ForgeVersionMeta, InstallerInfo,
};
use libmcmeta::models::mojang::MojangVersion;
use libmcmeta::models::MetaMcIndexEntry;

lazy_static! {
    pub static ref BAD_FORGE_VERSIONS: Vec<&'static str> = vec!["1.12.2-14.23.5.2851"];
}

fn process_results<T>(results: Vec<Result<T>>) -> Result<Vec<T>> {
    let mut err_flag = false;

    let ok_results: Vec<T> = results
        .into_iter()
        .filter_map(|res: Result<T>| match res {
            Err(err) => {
                error!("{:?}", err);
                err_flag = true;
                None
            }
            Ok(ok_res) => Some(ok_res),
        })
        .collect();
    if err_flag {
        Err(anyhow!("There were errors in the results"))
    } else {
        Ok(ok_results)
    }
}

fn process_results_ok<T>(results: Vec<Result<T>>) -> Vec<T> {
    results
        .into_iter()
        .filter_map(|res: Result<T>| res.ok())
        .collect()
}

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
            update_forge_metadata_json(metadata_cfg, meta_directory)
                .await
                .with_context(|| "Failed to update Forge metadata in the json format")?;
            update_forge_legacy_metadata_json(metadata_cfg, meta_directory)
                .await
                .with_context(|| "Failed to update Forge legacy metadata in the json format")
        }
        StorageFormat::Database => todo!(),
    }
}

async fn update_forge_metadata_json(
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

    let maven_metadata = download::forge::load_maven_metadata().await?;
    let promotions_metadata = download::forge::load_maven_promotions().await?;

    let promoted_key_expression = regex::Regex::new(
        "(?P<mc>[^-]+)-(?P<promotion>(latest)|(recommended))(-(?P<branch>[a-zA-Z0-9\\.]+))?",
    )
    .expect("Promotion regex must compile");

    let mut recommended_set = HashSet::new();

    let mut new_index = DerivedForgeIndex::default();

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
    let forge_version_pairs = maven_metadata
        .versions
        .iter()
        .flat_map(|(k, v)| v.iter().map(|lv| (k.clone(), lv.clone())))
        .collect::<Vec<_>>();
    let tasks = stream::iter(forge_version_pairs)
        .map(|(mc_version, long_version)| {
            let version_expression = regex::Regex::new(
                "^(?P<mc>[0-9a-zA-Z_\\.]+)-(?P<ver>[0-9\\.]+\\.(?P<build>[0-9]+))(-(?P<branch>[a-zA-Z0-9\\.]+))?$"
            ).expect("Version regex must compile");
            let forge_dir =  forge_meta_dir.clone();
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
                                &forge_dir,
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
    let forge_versions = process_results(results)?;

    for forge_version in forge_versions {
        let mc_version = forge_version.mc_version.clone();
        let long_version = forge_version.long_version.clone();
        new_index
            .versions
            .insert(forge_version.long_version.clone(), forge_version.clone());
        if !new_index.by_mc_version.contains_key(&mc_version) {
            new_index
                .by_mc_version
                .insert(mc_version.clone(), ForgeMCVersionInfo::default());
        }
        new_index
            .by_mc_version
            .get_mut(&mc_version)
            .unwrap_or_else(|| panic!("Missing forge info for minecraft version {}", &mc_version))
            .versions
            .push(long_version.clone());
        // NOTE: we add this later after the fact. The forge promotions file lies about these.
        // if let Some(true) = forge_version.latest {
        //     new_index.by_mc_version[&mc_version].latest = Some(long_version.clone());
        // }
        if let Some(true) = forge_version.recommended {
            new_index
                .by_mc_version
                .get_mut(&mc_version)
                .unwrap_or_else(|| {
                    panic!("Missing forge info for minecraft version {}", &mc_version)
                })
                .recommended = Some(long_version.clone());
        }
    }

    debug!("Post-processing forge promotions and adding missing 'latest'");

    for (mc_version, info) in &mut new_index.by_mc_version {
        let latest_version = info
            .versions
            .last()
            .unwrap_or_else(|| panic!("No forge versions for minecraft version {}", mc_version));
        info.latest = Some(latest_version.to_string());
        info!("Added {} as latest for {}", latest_version, mc_version)
    }

    debug!("Dumping forge index files");

    {
        let local_maven_metadata_file = forge_meta_dir.join("maven-metadata.json");
        let maven_metadata_json = serde_json::to_string_pretty(&maven_metadata)?;
        std::fs::write(&local_maven_metadata_file, maven_metadata_json).with_context(|| {
            format!(
                "Failure writing to file {}",
                &local_maven_metadata_file.to_string_lossy()
            )
        })?;
    }

    {
        let local_promotions_metadata_file = forge_meta_dir.join("promotions_slim.json");
        let promotions_metadata_json = serde_json::to_string_pretty(&promotions_metadata)?;
        std::fs::write(&local_promotions_metadata_file, promotions_metadata_json).with_context(
            || {
                format!(
                    "Failure writing to file {}",
                    &local_promotions_metadata_file.to_string_lossy()
                )
            },
        )?;
    }

    {
        let local_derived_index_file = forge_meta_dir.join("derived_index.json");
        let derived_index_json = serde_json::to_string_pretty(&new_index)?;
        std::fs::write(&local_derived_index_file, derived_index_json).with_context(|| {
            format!(
                "Failure writing to file {}",
                &local_derived_index_file.to_string_lossy()
            )
        })?;
    }

    Ok(())
}

async fn update_forge_legacy_metadata_json(
    metadata_cfg: &MetadataConfig,
    meta_directory: &str,
) -> Result<()> {
    let metadata_dir = std::path::Path::new(meta_directory);
    let forge_meta_dir = metadata_dir.join("forge");
    let static_dir = std::path::Path::new(&metadata_cfg.static_directory);
    let legacy_info_path = static_dir.join("forge").join("forge-legacyinfo.json");

    let mut legacy_info_list = ForgeLegacyInfoList::default();

    debug!("Grabbing forge installers and dumping installer profiles...");

    let derived_index_file = forge_meta_dir.join("derived_index.json");
    let derived_index = serde_json::from_str::<DerivedForgeIndex>(
        &std::fs::read_to_string(&derived_index_file).with_context(|| {
            format!("Failure opening {}", &derived_index_file.to_string_lossy())
        })?,
    )
    .with_context(|| {
        format!(
            "Failure reading json from {}",
            &derived_index_file.to_string_lossy()
        )
    })?;

    let derived_index_hash = filehash(&derived_index_file, HashAlgo::Sha256)?;

    let last_index_path = forge_meta_dir.join("derived_index.last_index.json");
    if last_index_path.is_file() {
        if let Ok(last_index) = serde_json::from_str::<MetaMcIndexEntry>(
            &std::fs::read_to_string(&last_index_path).with_context(|| {
                format!("Failure opening {}", &last_index_path.to_string_lossy())
            })?,
        ) {
            // check if we even need to regenerate
            if last_index.hash == derived_index_hash {
                info!("Forge index up to date. Not regenerating.");
                return Ok(());
            } else {
                info!("Forge index hash did not match, regenerating...")
            }
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
            let fm_dir = forge_meta_dir.clone();
            let li_path = legacy_info_path.clone();
            tokio::spawn(async move {
                process_legecy_forge_version(&version, fm_dir.as_path(), li_path.as_path()).await
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
        path: derived_index_file.to_str().unwrap().to_string(),
        hash: derived_index_hash,
    };
    let last_index_json = serde_json::to_string_pretty(&last_index)?;
    std::fs::write(&last_index_path, last_index_json).with_context(|| {
        format!(
            "Failure writing to file {}",
            &last_index_path.to_string_lossy()
        )
    })?;

    Ok(())
}

async fn process_forge_version(
    forge_meta_dir: &Path,
    recommended_set: &HashSet<String>,
    mc_version: &str,
    long_version: &str,
    build: i32,
    version: &str,
    branch: Option<String>,
) -> Result<ForgeEntry> {
    let files = get_single_forge_files_manifest(forge_meta_dir, long_version).await?;

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
    forge_meta_dir: &Path,
    long_version: &str,
) -> Result<BTreeMap<String, ForgeFile>> {
    info!("Getting Forge manifest for {long_version}");

    let forge_file_manifest_path = forge_meta_dir.join("files_manifests");

    if !forge_file_manifest_path.exists() {
        info!(
            "Forge files manifests directory at {} does not exist, creating it",
            forge_file_manifest_path.display()
        );
        std::fs::create_dir_all(&forge_file_manifest_path)?;
    }

    let files_manifest_file = forge_file_manifest_path.join(format!("{}.json", long_version));

    let mut from_file = false;

    let files_metadata = if files_manifest_file.is_file() {
        from_file = true;
        serde_json::from_str::<ForgeVersionMeta>(&std::fs::read_to_string(&files_manifest_file)?)?
    } else {
        let file_url = format!(
            "https://files.minecraftforge.net/net/minecraftforge/forge/{}/meta.json",
            &long_version
        );
        download::forge::load_single_forge_files_manifest(&file_url)
            .await
            .with_context(|| format!("Failure downloading {}", &file_url))?
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

    if !from_file {
        let files_metadata_json = serde_json::to_string_pretty(&files_metadata)?;
        std::fs::write(&files_manifest_file, files_metadata_json).with_context(|| {
            format!(
                "Failure writing to file {}",
                &files_manifest_file.to_string_lossy()
            )
        })?;
    }

    Ok(ret_map)
}

async fn process_legecy_forge_version(
    version: &ForgeProcessedVersion,
    forge_meta_dir: &Path,
    legacy_info_path: &Path,
) -> Result<Option<(String, ForgeLegacyInfo)>> {
    let jar_path = forge_meta_dir
        .join("jars")
        .join(&version.filename().expect("Missing forge filename"));

    let version_json_name = format!("{}.json", &version.long_version);

    let installer_manifests_dir = forge_meta_dir.join("installer_manifests");
    if !installer_manifests_dir.exists() {
        std::fs::create_dir_all(&installer_manifests_dir)
            .with_context(|| "Failed to create forge `installer_manifests` directory")?;
    }

    let version_manifests_dir = forge_meta_dir.join("version_manifests");
    if !version_manifests_dir.exists() {
        std::fs::create_dir_all(&version_manifests_dir)
            .with_context(|| "Failed to create forge `version_manifests` directory")?;
    }

    let installer_info_dir = forge_meta_dir.join("installer_info");
    if !installer_info_dir.exists() {
        std::fs::create_dir_all(&installer_info_dir)
            .with_context(|| "Failed to create forge `installer_info` directory")?;
    }

    if version.uses_installer() {
        let installer_info_path = installer_info_dir.join(&version_json_name);
        let profile_path = installer_manifests_dir.join(&version_json_name);
        let version_file_path = version_manifests_dir.join(&version_json_name);

        let installer_refresh_required = !profile_path.is_file() || !installer_info_path.is_file();

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
        if !profile_path.is_file() {
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

                    let version_file_json = serde_json::to_string_pretty(&mojang_version)?;
                    std::fs::write(&version_file_path, version_file_json).with_context(|| {
                        format!(
                            "Failure writing to file {}",
                            &version_file_path.to_string_lossy()
                        )
                    })?;
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

                let forge_profile_json: Result<String> = {
                    let forge_profile =
                        serde_json::from_str::<ForgeInstallerProfile>(&install_profile_data);
                    if let Ok(profile) = forge_profile {
                        Ok(serde_json::to_string_pretty(&profile)?)
                    } else {
                        let forge_profile_v2 =
                            serde_json::from_str::<ForgeInstallerProfileV2>(&install_profile_data);
                        if let Ok(profile) = forge_profile_v2 {
                            Ok(serde_json::to_string_pretty(&profile)?)
                        } else {
                            Err(forge_profile_v2.unwrap_err().into())
                        }
                    }
                };

                if let Ok(forge_profile_json) = forge_profile_json {
                    std::fs::write(&profile_path, forge_profile_json).with_context(|| {
                        format!(
                            "Failure writing to file {}",
                            &profile_path.to_string_lossy()
                        )
                    })?;
                } else if version.is_supported() {
                    return Err(forge_profile_json.unwrap_err()).with_context(|| {
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

        if !installer_info_path.is_file() {
            let installer_info = InstallerInfo {
                sha1hash: Some(filehash(&jar_path, HashAlgo::Sha1)?),
                sha256hash: Some(filehash(&jar_path, HashAlgo::Sha256)?),
                size: Some(jar_path.metadata()?.len()),
            };

            let installer_info_json = serde_json::to_string_pretty(&installer_info)?;
            std::fs::write(&installer_info_path, installer_info_json).with_context(|| {
                format!(
                    "Failure writing to file {}",
                    &installer_info_path.to_string_lossy()
                )
            })?;
        }
        Ok(None)
    } else {
        // ignore the two versions without install manifests and jar mod class files
        // TODO: fix those versions?

        if version.mc_version_sane == "1.6.1" {
            return Ok(None);
        }

        // only gather legacy info if it's missing
        if !legacy_info_path.is_file() {
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

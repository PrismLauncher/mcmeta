use anyhow::{anyhow, Context, Result};
use futures::{stream, StreamExt};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

use crate::{
    app_config::MetadataConfig,
    download,
    storage::StorageFormat,
    utils::{filehash, HashAlgo},
};
use libmcmeta::models::forge::{
    DerivedForgeIndex, ForgeEntry, ForgeFile, ForgeInstallerProfile, ForgeInstallerProfileV2,
    ForgeLegacyInfoList, ForgeMCVersionInfo, ForgeProcessedVersion, ForgeVersionMeta,
    InstallerInfo,
};
use libmcmeta::models::mojang::MojangVersion;

lazy_static! {
    pub static ref BAD_FORGE_VERSIONS: Vec<&'static str> = vec!["1.12.2-14.23.5.2851"];
}

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
        match promoted_key_expression.captures(&promo_key) {
            None => {
                warn!("Skipping promotion {}, the key did not parse:", promo_key);
            }
            Some(captures) => {
                if let None = captures.name("mc") {
                    debug!(
                        "Skipping promotion {}, because it has no Minecraft version.",
                        promo_key
                    );
                    continue;
                }
                if let Some(_) = captures.name("branch") {
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
        .map(|(k, v)| v.iter().map(|lv| (k.clone(), lv.clone())))
        .flatten()
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
                        if let None = captures.name("mc") {
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
                                captures.name("build").unwrap().as_str().parse::<i32>()?,
                                captures.name("ver").unwrap().as_str(),
                                captures.name("branch").unwrap().as_str(),
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
            .expect(&format!(
                "Missing forge info for minecraft version {}",
                &mc_version
            ))
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
                .expect(&format!(
                    "Missing forge info for minecraft version {}",
                    &mc_version
                ))
                .recommended = Some(long_version.clone());
        }
    }

    debug!("Post-processing forge promotions and adding missing 'latest'");

    for (mc_version, info) in &mut new_index.by_mc_version {
        let latest_version = info.versions.last().expect(&format!(
            "No forge versions for minecraft version {}",
            mc_version
        ));
        info.latest = Some(latest_version.to_string());
        info!("Added {} as latest for {}", latest_version, mc_version)
    }

    debug!("Dumping forge index files");

    {
        let local_maven_metadata_file = forge_meta_dir.join("maven-metadata.json");
        let maven_metadata_json = serde_json::to_string_pretty(&maven_metadata)?;
        std::fs::write(&local_maven_metadata_file, maven_metadata_json)?;
    }

    {
        let local_promotions_metadata_file = forge_meta_dir.join("promotions_slim.json");
        let promotions_metadata_json = serde_json::to_string_pretty(&promotions_metadata)?;
        std::fs::write(&local_promotions_metadata_file, promotions_metadata_json)?;
    }

    {
        let local_derived_index_file = forge_meta_dir.join("derived_index.json");
        let derived_index_json = serde_json::to_string_pretty(&new_index)?;
        std::fs::write(&local_derived_index_file, derived_index_json)?;
    }

    Ok(())
}

async fn update_forge_legacy_metadata_json(
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

    let mut legacy_info_list = ForgeLegacyInfoList::default();

    debug!("Grabbing forge installers and dumping installer profiles...");

    let derived_index_file = forge_meta_dir.join("derived_index.json");
    let derived_index =
        serde_json::from_str::<DerivedForgeIndex>(&std::fs::read_to_string(&derived_index_file)?)?;

    // get the installer jars - if needed - and get the installer profiles out of them
    for (key, entry) in derived_index.versions {
        debug!("Updating Forge {}", &key);
        let version = ForgeProcessedVersion::new(&entry);

        if version.url().is_none() {
            debug!("Skipping forge build {} with no valid files", &entry.build);
            continue;
        }

        if BAD_FORGE_VERSIONS.contains(&version.long_version.as_str()) {
            debug!("Skipping bad forge version {}", &version.long_version);
            continue;
        }

        let jar_path = forge_meta_dir
            .join("jars")
            .join(&version.filename().expect("Missing forge filename"));

        let version_json_name = format!("{}.json", &version.long_version);

        if version.uses_installer() {
            let installer_info_path = forge_meta_dir
                .join("installer_info")
                .join(&version_json_name);
            let profile_path = forge_meta_dir
                .join("installer_manifests")
                .join(&version_json_name);
            let version_file_path = forge_meta_dir
                .join("version_manifests")
                .join(&version_json_name);

            let installer_refresh_required =
                !profile_path.exists() || !installer_info_path.exists();

            if installer_refresh_required {
                // grab the installer if it's not there
                if !jar_path.exists() {
                    debug!("Downloading forge jar from {}", &version.url().unwrap());
                    download::download_binary_file(&jar_path, &version.url().unwrap()).await?
                }
            }

            debug!("Processing forge jar from {}", &version.url().unwrap());
            if !profile_path.is_file() {
                use std::io::Read;

                let mut jar = zip::ZipArchive::new(std::fs::File::open(&jar_path)?)?;

                {
                    // version.json
                    let mut version_zip_entry = jar.by_name("version.json")?;
                    let mut version_data = String::new();
                    version_zip_entry.read_to_string(&mut version_data)?;

                    let mojang_version: MojangVersion = serde_json::from_str(&version_data)?;

                    let version_file_json = serde_json::to_string_pretty(&mojang_version)?;
                    std::fs::write(&version_file_path, version_file_json)?;
                }

                {
                    //install_profile.json
                    let mut profile_zip_entry = jar.by_name("install_profile.json")?;
                    let mut install_profile_data = String::new();
                    profile_zip_entry.read_to_string(&mut install_profile_data)?;

                    let forge_profile_json: Result<String> = {
                        let forge_profile =
                            serde_json::from_str::<ForgeInstallerProfile>(&install_profile_data);
                        if let Ok(profile) = forge_profile {
                            Ok(serde_json::to_string_pretty(&profile)?)
                        } else {
                            let forge_profile_v2 = serde_json::from_str::<ForgeInstallerProfileV2>(
                                &install_profile_data,
                            );
                            if let Ok(profile) = forge_profile_v2 {
                                Ok(serde_json::to_string_pretty(&profile)?)
                            } else {
                                Err(forge_profile_v2.unwrap_err().into())
                            }
                        }
                    };

                    if let Ok(forge_profile_json) = forge_profile_json {
                        std::fs::write(&profile_path, forge_profile_json)?;
                    } else {
                        if version.is_supported() {
                            return Err(forge_profile_json.unwrap_err());
                        } else {
                            debug!(
                                "Forge Version {} is not supported and won't be generated later.",
                                &version.long_version
                            )
                        }
                    }
                }
            }

            if !installer_info_path.is_file() {
                let mut installer_info = InstallerInfo::default();
                installer_info.sha1hash = Some(filehash(&jar_path, HashAlgo::Sha1)?);
                installer_info.sha256hash = Some(filehash(&jar_path, HashAlgo::Sha256)?);
                installer_info.size = Some(jar_path.metadata()?.len());
            }
        } else {
            // ignore the two versions without install manifests and jar mod class files

            // # TODO: fix those versions?
            // if version.mc_version_sane == "1.6.1":
            //     continue

            // # only gather legacy info if it's missing
            // if not os.path.isfile(LEGACYINFO_PATH):
            //     # grab the jar/zip if it's not there
            //     if not os.path.isfile(jar_path):
            //         rfile = sess.get(version.url(), stream=True)
            //         rfile.raise_for_status()
            //         with open(jar_path, "wb") as f:
            //             for chunk in rfile.iter_content(chunk_size=128):
            //                 f.write(chunk)
            //     # find the latest timestamp in the zip file
            //     tstamp = datetime.fromtimestamp(0)
            //     with zipfile.ZipFile(jar_path) as jar:
            //         for info in jar.infolist():
            //             tstamp_new = datetime(*info.date_time)
            //             if tstamp_new > tstamp:
            //                 tstamp = tstamp_new
            //     legacy_info = ForgeLegacyInfo()
            //     legacy_info.release_time = tstamp
            //     legacy_info.sha1 = filehash(jar_path, hashlib.sha1)
            //     legacy_info.sha256 = filehash(jar_path, hashlib.sha256)
            //     legacy_info.size = os.path.getsize(jar_path)
            //     legacy_info_list.number[key] = legacy_info
        }
    }

    // only write legacy info if it's missing

    // if not os.path.isfile(LEGACYINFO_PATH):
    //     legacy_info_list.write(LEGACYINFO_PATH)

    Ok(())
}

async fn process_forge_version(
    forge_meta_dir: &PathBuf,
    recommended_set: &HashSet<String>,
    mc_version: &str,
    long_version: &str,
    build: i32,
    version: &str,
    branch: &str,
) -> Result<ForgeEntry> {
    let files = get_single_forge_files_manifest(forge_meta_dir, long_version).await?;

    let is_recommended = recommended_set.contains(version);

    let entry = ForgeEntry {
        long_version: long_version.to_string(),
        mc_version: mc_version.to_string(),
        version: version.to_string(),
        build: build,
        branch: Some(branch.to_string()),
        latest: None, // NOTE: we add this later after the fact. The forge promotions file lies about these.
        recommended: Some(is_recommended),
        files: Some(files),
    };

    Ok(entry)
}

async fn get_single_forge_files_manifest(
    forge_meta_dir: &PathBuf,
    long_version: &str,
) -> Result<HashMap<String, ForgeFile>> {
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

    let files_metadata = if files_manifest_file.exists() {
        from_file = true;
        serde_json::from_str::<ForgeVersionMeta>(&std::fs::read_to_string(&files_manifest_file)?)?
    } else {
        let file_url = format!(
            "https://files.minecraftforge.net/net/minecraftforge/forge/{}/meta.json",
            &long_version
        );
        download::forge::load_single_forge_files_manifest(&file_url).await?
    };

    let mut ret_map: HashMap<String, ForgeFile> = HashMap::new();

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
        std::fs::write(&files_manifest_file, files_metadata_json)?;
    }

    Ok(ret_map)
}

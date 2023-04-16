use anyhow::{anyhow, Context, Result};
use futures::{stream, StreamExt};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

use libmcmeta::models::forge::{
    DerivedForgeIndex, ForgeEntry, ForgeFile, ForgeMCVersionInfo, ForgeVersionMeta,
};

use crate::{app_config::MetadataConfig, download, storage::StorageFormat};

fn process_results<T>(results: Vec<Result<T>>) -> Result<Vec<T>> {
    let err_flag = false;
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
            return initialize_forge_metadata_json(metadata_cfg, meta_directory)
                .await
                .with_context(|| "Failed to initializ Forge metadata in the json format")
        }
        StorageFormat::Database => todo!(),
    }
}

async fn initialize_forge_metadata_json(
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

    let main_metadata = download::forge::load_maven_metadata().await?;
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

    for (promo_key, shortversion) in promotions_metadata.promos {
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

    let version_expression = regex::Regex::new(
        "^(?P<mc>[0-9a-zA-Z_\\.]+)-(?P<ver>[0-9\\.]+\\.(?P<build>[0-9]+))(-(?P<branch>[a-zA-Z0-9\\.]+))?$"
    ).expect("Version regex must compile");

    debug!("Processing Forge Versions");

    // let tasks = vec![];

    let forge_version_pairs = main_metadata
        .versions
        .iter()
        .map(|(k, v)| v.iter().map(|lv| (k.clone(), lv.clone())))
        .flatten()
        .collect::<Vec<_>>();
    let tasks = stream::iter(forge_version_pairs)
        .map(|(mc_version, long_version)| {
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
                            let build = captures.name("build").unwrap().as_str().parse::<i32>()?;
                            let version = captures.name("ver").unwrap().as_str();
                            let branch = captures.name("branch").unwrap().as_str();

                            process_forge_version(
                                &forge_meta_dir,
                                &recommended_set,
                                &mc_version,
                                &long_version,
                                build,
                                version,
                                branch,
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
        new_index.versions[&forge_version.long_version] = forge_version;
        if !new_index.by_mc_version.contains_key(&mc_version) {
            new_index.by_mc_version[&mc_version] = ForgeMCVersionInfo::default();
        }
        new_index.by_mc_version[&mc_version]
            .versions
            .push(long_version);
        // NOTE: we add this later after the fact. The forge promotions file lies about these.
        // if let Some(true) = forge_version.latest {
        //     new_index.by_mc_version[&mc_version].latest = Some(long_version.clone());
        // }
        if let Some(true) = forge_version.recommended {
            new_index.by_mc_version[&mc_version].recommended = Some(long_version.clone());
        }
    }

    // print("")
    // print("Post processing promotions and adding missing 'latest':")
    // for mc_version, info in new_index.by_mc_version.items():
    //     latest_version = info.versions[-1]
    //     info.latest = latest_version
    //     new_index.versions[latest_version].latest = True
    //     print("Added %s as latest for %s" % (latest_version, mc_version))

    // print("")
    // print("Dumping index files...")

    // with open(UPSTREAM_DIR + "/forge/maven-metadata.json", "w", encoding="utf-8") as f:
    //     json.dump(main_json, f, sort_keys=True, indent=4)

    // with open(UPSTREAM_DIR + "/forge/promotions_slim.json", "w", encoding="utf-8") as f:
    //     json.dump(promotions_json, f, sort_keys=True, indent=4)

    // new_index.write(UPSTREAM_DIR + "/forge/derived_index.json")

    // legacy_info_list = ForgeLegacyInfoList()

    // print("Grabbing installers and dumping installer profiles...")
    // # get the installer jars - if needed - and get the installer profiles out of them
    // for key, entry in new_index.versions.items():
    //     eprint("Updating Forge %s" % key)
    //     if entry.mc_version is None:
    //         eprint("Skipping %d with invalid MC version" % entry.build)
    //         continue

    //     version = ForgeVersion(entry)
    //     if version.url() is None:
    //         eprint("Skipping %d with no valid files" % version.build)
    //         continue
    //     if version.long_version in BAD_VERSIONS:
    //         eprint(f"Skipping bad version {version.long_version}")
    //         continue

    //     jar_path = os.path.join(UPSTREAM_DIR, JARS_DIR, version.filename())

    //     if version.uses_installer():
    //         installer_info_path = (
    //             UPSTREAM_DIR + "/forge/installer_info/%s.json" % version.long_version
    //         )
    //         profile_path = (
    //             UPSTREAM_DIR
    //             + "/forge/installer_manifests/%s.json" % version.long_version
    //         )
    //         version_file_path = (
    //             UPSTREAM_DIR + "/forge/version_manifests/%s.json" % version.long_version
    //         )

    //         installer_refresh_required = not os.path.isfile(
    //             profile_path
    //         ) or not os.path.isfile(installer_info_path)

    //         if installer_refresh_required:
    //             # grab the installer if it's not there
    //             if not os.path.isfile(jar_path):
    //                 eprint("Downloading %s" % version.url())
    //                 rfile = sess.get(version.url(), stream=True)
    //                 rfile.raise_for_status()
    //                 with open(jar_path, "wb") as f:
    //                     for chunk in rfile.iter_content(chunk_size=128):
    //                         f.write(chunk)

    //         eprint("Processing %s" % version.url())
    //         # harvestables from the installer
    //         if not os.path.isfile(profile_path):
    //             print(jar_path)
    //             with zipfile.ZipFile(jar_path) as jar:
    //                 with suppress(KeyError):
    //                     with jar.open("version.json") as profile_zip_entry:
    //                         version_data = profile_zip_entry.read()

    //                         # Process: does it parse?
    //                         MojangVersion.parse_raw(version_data)

    //                         with open(version_file_path, "wb") as versionJsonFile:
    //                             versionJsonFile.write(version_data)
    //                             versionJsonFile.close()

    //                 with jar.open("install_profile.json") as profile_zip_entry:
    //                     install_profile_data = profile_zip_entry.read()

    //                     # Process: does it parse?
    //                     is_parsable = False
    //                     exception = None
    //                     try:
    //                         ForgeInstallerProfile.parse_raw(install_profile_data)
    //                         is_parsable = True
    //                     except ValidationError as err:
    //                         exception = err
    //                     try:
    //                         ForgeInstallerProfileV2.parse_raw(install_profile_data)
    //                         is_parsable = True
    //                     except ValidationError as err:
    //                         exception = err

    //                     if not is_parsable:
    //                         if version.is_supported():
    //                             raise exception
    //                         else:
    //                             eprint(
    //                                 "Version %s is not supported and won't be generated later."
    //                                 % version.long_version
    //                             )

    //                     with open(profile_path, "wb") as profileFile:
    //                         profileFile.write(install_profile_data)
    //                         profileFile.close()

    //         # installer info v1
    //         if not os.path.isfile(installer_info_path):
    //             installer_info = InstallerInfo()
    //             installer_info.sha1hash = filehash(jar_path, hashlib.sha1)
    //             installer_info.sha256hash = filehash(jar_path, hashlib.sha256)
    //             installer_info.size = os.path.getsize(jar_path)
    //             installer_info.write(installer_info_path)
    //     else:
    //         # ignore the two versions without install manifests and jar mod class files
    //         # TODO: fix those versions?
    //         if version.mc_version_sane == "1.6.1":
    //             continue

    //         # only gather legacy info if it's missing
    //         if not os.path.isfile(LEGACYINFO_PATH):
    //             # grab the jar/zip if it's not there
    //             if not os.path.isfile(jar_path):
    //                 rfile = sess.get(version.url(), stream=True)
    //                 rfile.raise_for_status()
    //                 with open(jar_path, "wb") as f:
    //                     for chunk in rfile.iter_content(chunk_size=128):
    //                         f.write(chunk)
    //             # find the latest timestamp in the zip file
    //             tstamp = datetime.fromtimestamp(0)
    //             with zipfile.ZipFile(jar_path) as jar:
    //                 for info in jar.infolist():
    //                     tstamp_new = datetime(*info.date_time)
    //                     if tstamp_new > tstamp:
    //                         tstamp = tstamp_new
    //             legacy_info = ForgeLegacyInfo()
    //             legacy_info.release_time = tstamp
    //             legacy_info.sha1 = filehash(jar_path, hashlib.sha1)
    //             legacy_info.sha256 = filehash(jar_path, hashlib.sha256)
    //             legacy_info.size = os.path.getsize(jar_path)
    //             legacy_info_list.number[key] = legacy_info

    // # only write legacy info if it's missing
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

    let is_recommended = recommended_set.contains(&version);

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
        let index = 0;
        let count = 0;

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
                            ret_map[classifier.as_str()] = file_obj;
                            count += 1;
                        } else {
                            return Err(anyhow!(
                                "{}: Multiple objects detected for classifier {}: {}",
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

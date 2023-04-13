use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use serde_with::skip_serializing_none;
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
pub struct ForgeMavenMetadata {
    #[serde(flatten)]
    pub versions: HashMap<String, Vec<String>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
pub struct ForgeMavenPromotions {
    pub homepage: String,
    pub promos: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
pub struct ForgeVersionMeta {
    pub classifiers: ForgeVersionClassifiers,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(deny_unknown_fields)]
pub struct ForgeVersionClassifier {
    pub txt: Option<String>,
    pub zip: Option<String>,
    pub jar: Option<String>,
    pub stash: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(deny_unknown_fields)]
pub struct ForgeVersionClassifiers {
    pub changelog: Option<ForgeVersionClassifier>,
    pub installer: Option<ForgeVersionClassifier>,
    pub mdk: Option<ForgeVersionClassifier>,
    pub universal: Option<ForgeVersionClassifier>,
    pub userdev: Option<ForgeVersionClassifier>,
    pub sources: Option<ForgeVersionClassifier>,
    pub javadoc: Option<ForgeVersionClassifier>,
    pub client: Option<ForgeVersionClassifier>,
    pub src: Option<ForgeVersionClassifier>,
    pub server: Option<ForgeVersionClassifier>,
    pub launcher: Option<ForgeVersionClassifier>,
    pub userdev3: Option<ForgeVersionClassifier>,
    #[serde(rename = "src.zip")]
    pub src_zip: Option<ForgeVersionClassifier>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(deny_unknown_fields)]
pub struct ForgeVersionArguments {
    pub game: Vec<String>,
    pub jvm: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct ForgeVersionLibraryArtifact {
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct ForgeVersionLibraryDownloads {
    pub artifact: ForgeVersionLibraryArtifact,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct ForgeVersionLibrary {
    pub name: String,
    pub downloads: ForgeVersionLibraryDownloads,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct ForgeVersionLoggingFile {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct ForgeVersionLoggingClient {
    pub argument: String,
    pub file: ForgeVersionLoggingFile,
    #[serde(rename = "type")]
    pub client_type: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct ForgeVersionLogging {
    pub client: Option<ForgeVersionLoggingClient>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForgeVersion {
    #[serde(rename = "_comment_")]
    pub comment: Option<Vec<String>>,
    pub id: String,
    pub time: String,
    pub release_time: String,
    #[serde(rename = "type")]
    pub release_type: String,
    pub main_class: String,
    pub inherits_from: String,
    pub logging: ForgeVersionLogging,
    pub arguments: Option<ForgeVersionArguments>,
    pub libraries: Vec<ForgeVersionLibrary>,
    pub minecraft_arguments: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct ForgeInstallerDataInfo {
    pub client: String,
    pub server: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(deny_unknown_fields, rename_all = "SCREAMING_SNAKE_CASE")]
pub struct ForgeInstallerData {
    pub mappings: Option<ForgeInstallerDataInfo>,
    pub mojmaps: Option<ForgeInstallerDataInfo>,
    pub merged_mappings: Option<ForgeInstallerDataInfo>,
    pub binpatch: Option<ForgeInstallerDataInfo>,
    pub mc_unpacked: Option<ForgeInstallerDataInfo>,
    pub mc_slim: Option<ForgeInstallerDataInfo>,
    pub mc_slim_sha: Option<ForgeInstallerDataInfo>,
    pub mc_extra: Option<ForgeInstallerDataInfo>,
    pub mc_extra_sha: Option<ForgeInstallerDataInfo>,
    pub mc_srg: Option<ForgeInstallerDataInfo>,
    pub patched: Option<ForgeInstallerDataInfo>,
    pub patched_sha: Option<ForgeInstallerDataInfo>,
    pub mcp_version: Option<ForgeInstallerDataInfo>,
    pub mc_data: Option<ForgeInstallerDataInfo>,
    pub mc_data_sha: Option<ForgeInstallerDataInfo>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(deny_unknown_fields)]
pub struct ForgeInstallerProcessor {
    pub sides: Option<Vec<String>>,
    pub jar: String,
    pub classpath: Vec<String>,
    pub args: Vec<String>,
    pub outputs: Option<HashMap<String, String>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct ForgeLegacyLogging {}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForgeLegacyInstall {
    pub profile_name: String,
    pub target: String,
    pub path: String,
    pub version: String,
    pub file_path: String,
    pub welcome: String,
    pub minecraft: String,
    pub mirror_list: String,
    pub logo: String,
    pub mod_list: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForgeLegacyLibraryNatives {
    pub linux: Option<String>,
    pub osx: Option<String>,
    pub windows: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForgeLegacyLibraryExtract {
    pub exclude: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestRule {
    pub action: String,
    pub os: Option<ManifestRuleOS>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestRuleOS {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForgeLegacyLibrary {
    pub name: String,
    pub url: Option<String>,
    pub serverreq: Option<bool>,
    pub clientreq: Option<bool>,
    pub checksums: Option<Vec<String>>,
    pub natives: Option<ForgeLegacyLibraryNatives>,
    pub extract: Option<ForgeLegacyLibraryExtract>,
    pub rules: Option<Vec<ManifestRule>>,
    pub comment: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForgeLegacyVersionInfo {
    pub id: String,
    pub time: String,
    pub release_time: String,
    #[serde(rename = "type")]
    pub release_type: String,
    pub minecraft_arguments: String,
    pub minimum_launcher_version: Option<u64>,
    pub assets: Option<String>,
    pub main_class: String,
    pub libraries: Vec<ForgeLegacyLibrary>,
    pub inherits_from: Option<String>,
    pub process_arguments: Option<String>,
    pub jar: Option<String>,
    pub logging: Option<ForgeLegacyLogging>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(deny_unknown_fields)]
pub struct ForgeLegacyOptional {
    pub name: String,
    pub client: bool,
    pub server: bool,
    pub default: bool,
    pub inject: bool,
    pub desc: String,
    pub url: String,
    pub artifact: String,
    pub maven: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForgeLegacyInstallerManifest {
    #[serde(rename = "_comment_")]
    pub comment: Option<Vec<String>>,
    pub install: ForgeLegacyInstall,
    pub version_info: ForgeLegacyVersionInfo,
    pub optionals: Option<Vec<ForgeLegacyOptional>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[skip_serializing_none]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForgeInstallerManifest {
    #[serde(rename = "_comment_")]
    pub comment: Option<Vec<String>>,
    pub spec: u64,
    pub profile: String,
    pub version: String,
    pub path: Option<String>,
    pub minecraft: String,
    pub server_jar_path: Option<String>,
    pub icon: Option<String>,
    pub json: String,
    pub logo: String,
    pub mirror_list: Option<String>,
    pub welcome: String,
    pub data: ForgeInstallerData,
    pub processors: Vec<ForgeInstallerProcessor>,
    pub libraries: Vec<ForgeVersionLibrary>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(untagged)]
pub enum ForgeInstallerManifestVersion {
    Legacy(Box<ForgeLegacyInstallerManifest>),
    Modern(Box<ForgeInstallerManifest>),
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_deserialization() {
        // meta dir is ./meta
        let cwd = std::env::current_dir().unwrap();
        let meta_dir = cwd.join("../meta/forge");
        println!("meta_dir: {:?}", meta_dir);

        let metadata_path = meta_dir.join("maven-metadata.json");
        let metadata = serde_json::from_str::<super::ForgeMavenMetadata>(
            &std::fs::read_to_string(metadata_path).unwrap(),
        );
        if let Err(e) = metadata {
            panic!("Failed to deserialize metadata: {:?}", e);
        }
        let metadata = metadata.unwrap();

        let promotions_path = meta_dir.join("promotions_slim.json");
        let promotions = serde_json::from_str::<super::ForgeMavenPromotions>(
            &std::fs::read_to_string(promotions_path).unwrap(),
        );
        if let Err(e) = promotions {
            panic!("Failed to deserialize promotions: {:?}", e);
        }

        for (_, forge_versions) in metadata.versions {
            for forge_version in forge_versions {
                let version_path = meta_dir.join(format!("files_manifests/{}.json", forge_version));
                let version = serde_json::from_str::<super::ForgeVersionMeta>(
                    &std::fs::read_to_string(version_path).unwrap(),
                );
                if let Err(e) = version {
                    panic!(
                        "Failed to deserialize file manifest of version {}: {:?}",
                        forge_version, e
                    );
                }
                let installer_path =
                    meta_dir.join(format!("installer_manifests/{}.json", forge_version));
                if installer_path.exists() {
                    let installer = serde_json::from_str::<super::ForgeInstallerManifestVersion>(
                        &std::fs::read_to_string(&installer_path).unwrap(),
                    );
                    if let Err(e) = installer {
                        panic!(
                            "Failed to deserialize installer manifest of version {}: {:?}",
                            forge_version, e
                        );
                    }
                }

                let version_path =
                    meta_dir.join(format!("version_manifests/{}.json", forge_version));
                if version_path.exists() {
                    let version = serde_json::from_str::<super::ForgeVersion>(
                        &std::fs::read_to_string(version_path).unwrap(),
                    );
                    if let Err(e) = version {
                        panic!(
                            "Failed to deserialize manifest for version {}: {:?}",
                            forge_version, e
                        );
                    }
                }
            }
        }
    }
}

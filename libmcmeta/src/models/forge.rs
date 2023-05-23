use crate::models::merge::{self, Merge};

use crate::models::{GradleSpecifier, MojangLibrary};
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use serde_with::skip_serializing_none;
use std::collections::{BTreeMap, HashMap};

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
pub enum ForgeVersionClassifierExtensions {
    Txt,
    Zip,
    Jar,
    Stash,
}

impl ForgeVersionClassifierExtensions {
    pub fn as_str(&self) -> &'static str {
        match self {
            ForgeVersionClassifierExtensions::Txt => "txt",
            ForgeVersionClassifierExtensions::Zip => "zip",
            ForgeVersionClassifierExtensions::Jar => "jar",
            ForgeVersionClassifierExtensions::Stash => "stash",
        }
    }
}

pub struct ForgeVersionClassifierIter<'a> {
    classifier: &'a ForgeVersionClassifier,
    index: usize,
}

impl ForgeVersionClassifier {
    pub fn iter(&self) -> ForgeVersionClassifierIter<'_> {
        ForgeVersionClassifierIter {
            classifier: self,
            index: 0,
        }
    }
}

impl<'a> Iterator for ForgeVersionClassifierIter<'a> {
    type Item = (ForgeVersionClassifierExtensions, &'a Option<String>);

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.index {
            0 => (ForgeVersionClassifierExtensions::Txt, &self.classifier.txt),
            1 => (ForgeVersionClassifierExtensions::Zip, &self.classifier.zip),
            2 => (ForgeVersionClassifierExtensions::Jar, &self.classifier.jar),
            3 => (
                ForgeVersionClassifierExtensions::Stash,
                &self.classifier.stash,
            ),
            _ => return None,
        };
        self.index += 1;
        Some(result)
    }
}

impl<'a> IntoIterator for &'a ForgeVersionClassifier {
    type Item = (ForgeVersionClassifierExtensions, &'a Option<String>);
    type IntoIter = ForgeVersionClassifierIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ForgeVersionClassifierIter {
            classifier: self,
            index: 0,
        }
    }
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

pub enum ForgeVersionClassifierNames {
    Changelog,
    Installer,
    Mdk,
    Universal,
    Userdev,
    Sources,
    Javadoc,
    Client,
    Src,
    Server,
    Launcher,
    Userdev3,
    SrcZip,
}

impl ForgeVersionClassifierNames {
    pub fn as_str(&self) -> &'static str {
        match self {
            ForgeVersionClassifierNames::Changelog => "changelog",
            ForgeVersionClassifierNames::Installer => "installer",
            ForgeVersionClassifierNames::Mdk => "mdk",
            ForgeVersionClassifierNames::Universal => "universal",
            ForgeVersionClassifierNames::Userdev => "userdev",
            ForgeVersionClassifierNames::Sources => "sources",
            ForgeVersionClassifierNames::Javadoc => "javadoc",
            ForgeVersionClassifierNames::Client => "client",
            ForgeVersionClassifierNames::Src => "src",
            ForgeVersionClassifierNames::Server => "server",
            ForgeVersionClassifierNames::Launcher => "launcher",
            ForgeVersionClassifierNames::Userdev3 => "userdev3",
            ForgeVersionClassifierNames::SrcZip => "src.zip",
        }
    }
}

pub struct ForgeVersionClassifiersIter<'a> {
    classifiers: &'a ForgeVersionClassifiers,
    index: usize,
}

impl ForgeVersionClassifiers {
    pub fn iter(&self) -> ForgeVersionClassifiersIter<'_> {
        ForgeVersionClassifiersIter {
            classifiers: self,
            index: 0,
        }
    }
}

impl<'a> IntoIterator for &'a ForgeVersionClassifiers {
    type Item = (
        ForgeVersionClassifierNames,
        &'a Option<ForgeVersionClassifier>,
    );
    type IntoIter = ForgeVersionClassifiersIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ForgeVersionClassifiersIter {
            classifiers: self,
            index: 0,
        }
    }
}

impl<'a> Iterator for ForgeVersionClassifiersIter<'a> {
    type Item = (
        ForgeVersionClassifierNames,
        &'a Option<ForgeVersionClassifier>,
    );

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.index {
            0 => (
                ForgeVersionClassifierNames::Changelog,
                &self.classifiers.changelog,
            ),
            1 => (
                ForgeVersionClassifierNames::Installer,
                &self.classifiers.installer,
            ),
            2 => (ForgeVersionClassifierNames::Mdk, &self.classifiers.mdk),
            3 => (
                ForgeVersionClassifierNames::Universal,
                &self.classifiers.universal,
            ),
            4 => (
                ForgeVersionClassifierNames::Userdev,
                &self.classifiers.userdev,
            ),
            5 => (
                ForgeVersionClassifierNames::Sources,
                &self.classifiers.sources,
            ),
            6 => (
                ForgeVersionClassifierNames::Javadoc,
                &self.classifiers.javadoc,
            ),
            7 => (
                ForgeVersionClassifierNames::Client,
                &self.classifiers.client,
            ),
            8 => (ForgeVersionClassifierNames::Src, &self.classifiers.src),
            9 => (
                ForgeVersionClassifierNames::Server,
                &self.classifiers.server,
            ),
            10 => (
                ForgeVersionClassifierNames::Launcher,
                &self.classifiers.launcher,
            ),
            11 => (
                ForgeVersionClassifierNames::Userdev3,
                &self.classifiers.userdev3,
            ),
            12 => (
                ForgeVersionClassifierNames::SrcZip,
                &self.classifiers.src_zip,
            ),
            _ => return None,
        };
        self.index += 1;
        Some(result)
    }
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
    #[serde(rename = "_comment_")]
    pub comment: Option<Vec<String>>,
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

#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge)]
#[serde(deny_unknown_fields)]
pub struct ForgeFile {
    #[merge(strategy = merge::overwrite)]
    pub classifier: String,
    #[merge(strategy = merge::overwrite)]
    pub hash: String,
    #[merge(strategy = merge::overwrite)]
    pub extension: String,
}

impl ForgeFile {
    pub fn filename(&self, long_version: &str) -> String {
        format!(
            "{}-{}-{}.{}",
            "forge", long_version, self.classifier, self.extension
        )
    }

    pub fn url(&self, long_version: &str) -> String {
        format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{}/{}",
            long_version,
            self.filename(long_version),
        )
    }
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
#[serde(deny_unknown_fields)]
pub struct ForgeEntry {
    #[serde(rename = "longversion")]
    #[merge(strategy = merge::overwrite)]
    pub long_version: String,
    #[serde(rename = "mcversion")]
    #[merge(strategy = merge::overwrite)]
    pub mc_version: String,
    #[merge(strategy = merge::overwrite)]
    pub version: String,
    #[merge(strategy = merge::overwrite)]
    pub build: i32,
    #[merge(strategy = merge::option::overwrite_some)]
    pub branch: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub latest: Option<bool>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub recommended: Option<bool>,
    #[merge(strategy = merge::option_btreemap::recurse_some)]
    pub files: Option<BTreeMap<String, ForgeFile>>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
#[serde(deny_unknown_fields)]
pub struct ForgeMCVersionInfo {
    #[merge(strategy = merge::option::overwrite_some)]
    pub latest: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub recommended: Option<String>,
    #[merge(strategy = merge::vec::append)]
    pub versions: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
#[serde(deny_unknown_fields)]
pub struct DerivedForgeIndex {
    #[merge(strategy = merge::btreemap::recurse)]
    pub versions: BTreeMap<String, ForgeEntry>,
    #[serde(rename = "by_mcversion")]
    #[merge(strategy = merge::btreemap::recurse)]
    pub by_mc_version: BTreeMap<String, ForgeMCVersionInfo>,
}

/// Example content
/// ```json
/// "install": {
///     "profileName": "Forge",
///     "target":"Forge8.9.0.753",
///     "path":"net.minecraftforge:minecraftforge:8.9.0.753",
///     "version":"Forge 8.9.0.753",
///     "filePath":"minecraftforge-universal-1.6.1-8.9.0.753.jar",
///     "welcome":"Welcome to the simple Forge installer.",
///     "minecraft":"1.6.1",
///     "logo":"/big_logo.png",
///     "mirrorList": "http://files.minecraftforge.net/mirror-brand.list"
/// },
/// "install": {
///     "profileName": "forge",
///     "target":"1.11-forge1.11-13.19.0.2141",
///     "path":"net.minecraftforge:forge:1.11-13.19.0.2141",
///     "version":"forge 1.11-13.19.0.2141",
///     "filePath":"forge-1.11-13.19.0.2141-universal.jar",
///     "welcome":"Welcome to the simple forge installer.",
///     "minecraft":"1.11",
///     "mirrorList" : "http://files.minecraftforge.net/mirror-brand.list",
///     "logo":"/big_logo.png",
///     "modList":"none"
/// },
/// ```
#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForgeInstallerProfileInstallSection {
    #[merge(strategy = merge::overwrite)]
    pub profile_name: String,
    #[merge(strategy = merge::overwrite)]
    pub target: String,
    #[merge(strategy = merge::overwrite)]
    pub path: GradleSpecifier,
    #[merge(strategy = merge::overwrite)]
    pub version: String,
    #[merge(strategy = merge::overwrite)]
    pub file_path: String,
    #[merge(strategy = merge::overwrite)]
    pub welcome: String,
    #[merge(strategy = merge::overwrite)]
    pub minecraft: String,
    #[merge(strategy = merge::overwrite)]
    pub logo: String,
    #[merge(strategy = merge::overwrite)]
    pub mirror_list: String,
    #[merge(strategy = merge::option::overwrite_some)]
    pub mod_list: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
#[serde(deny_unknown_fields)]
pub struct ForgeLibrary {
    #[merge(strategy = merge::overwrite)]
    pub url: Option<String>,
    #[serde(rename = "serverreq")]
    #[merge(strategy = merge::option::overwrite_some)]
    pub server_req: Option<bool>,
    #[serde(rename = "clientreq")]
    #[merge(strategy = merge::option::overwrite_some)]
    pub client_req: Option<bool>,
    #[merge(strategy = merge::option_vec::append_some)]
    pub checksums: Option<Vec<String>>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub comment: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForgeVersionFile {
    #[merge(strategy = merge::option_vec::append_some)]
    pub libraries: Option<Vec<ForgeLibrary>>, // overrides Mojang libraries
    #[merge(strategy = merge::option::overwrite_some)]
    pub inherits_from: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub jar: Option<String>,
}

/// Example content:
/// ```json
/// "optionals": [
///     {
///         "name": "Mercurius",
///         "client": true,
///         "server": true,
///         "default": true,
///         "inject": true,
///         "desc": "A mod that collects statistics about Minecraft and your system.<br>Useful for Forge to understand how Minecraft/Forge are used.",
///         "url": "http://www.minecraftforge.net/forum/index.php?topic=43278.0",
///         "artifact": "net.minecraftforge:MercuriusUpdater:1.11.2",
///         "maven": "http://maven.minecraftforge.net/"
///     }
/// ]
/// ```
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForgeOptional {
    #[merge(strategy = merge::option::overwrite_some)]
    pub name: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub client: Option<bool>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub server: Option<bool>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub default: Option<bool>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub inject: Option<bool>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub desc: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub url: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub artifact: Option<GradleSpecifier>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub maven: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
pub struct ForgeInstallerProfileV1 {
    pub install: ForgeInstallerProfileInstallSection,
    #[serde(rename = "versionInfo")]
    pub version_info: ForgeVersionFile,
    #[merge(strategy = merge::option_vec::append_some)]
    pub optionals: Option<Vec<ForgeOptional>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
pub struct ForgeLegacyInfo {
    #[merge(strategy = merge::option::overwrite_some)]
    #[serde(rename = "releaseTime", with = "time::serde::iso8601::option")]
    pub release_time: Option<time::OffsetDateTime>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub size: Option<u64>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub sha256: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub sha1: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
pub struct ForgeLegacyInfoList {
    #[merge(strategy = merge::hashmap::recurse)]
    pub number: HashMap<String, ForgeLegacyInfo>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
pub struct DataSpec {
    #[merge(strategy = merge::option::overwrite_some)]
    client: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    server: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
pub struct ProcessorSpec {
    #[merge(strategy = merge::option::overwrite_some)]
    jar: Option<String>,
    #[merge(strategy = merge::option_vec::append_some)]
    classpath: Option<Vec<String>>,
    #[merge(strategy = merge::option_vec::append_some)]
    args: Option<Vec<String>>,
    #[merge(strategy = merge::option_hashmap::overwrite_key_some)]
    outputs: Option<HashMap<String, String>>,
    #[merge(strategy = merge::option_vec::append_some)]
    sides: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
pub struct ForgeInstallerProfileV2 {
    #[merge(skip)]
    _comment: Option<Vec<String>>,
    #[merge(strategy = merge::option::overwrite_some)]
    spec: Option<i32>,
    #[merge(strategy = merge::option::overwrite_some)]
    profile: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    version: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    icon: Option<String>,
    #[serde(rename = "json")]
    #[merge(strategy = merge::option::overwrite_some)]
    json_data: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    path: Option<GradleSpecifier>,
    #[merge(strategy = merge::option::overwrite_some)]
    logo: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    minecraft: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    welcome: Option<String>,
    #[merge(strategy = merge::option_hashmap::recurse_some)]
    data: Option<HashMap<String, DataSpec>>,
    #[merge(strategy = merge::option_vec::append_some)]
    processors: Option<Vec<ProcessorSpec>>,
    #[merge(strategy = merge::option_vec::append_some)]
    libraries: Option<Vec<MojangLibrary>>,
    #[serde(rename = "mirrorList")]
    #[merge(strategy = merge::option::overwrite_some)]
    mirror_list: Option<String>,
    #[serde(rename = "erverJarPath")]
    #[merge(strategy = merge::option::overwrite_some)]
    server_jar_path: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate)]
#[serde(untagged)]
pub enum ForgeInstallerProfile {
    V1(Box<ForgeInstallerProfileV1>),
    V2(Box<ForgeInstallerProfileV2>),
}

#[derive(Deserialize, Serialize, Clone, Debug, Validate, Merge, Default)]
pub struct InstallerInfo {
    pub sha1hash: Option<String>,
    pub sha256hash: Option<String>,
    pub size: Option<u64>,
}

pub struct ForgeProcessedVersion {
    pub build: i32,
    pub raw_version: String,
    pub mc_version: String,
    pub mc_version_sane: String,
    pub branch: Option<String>,
    pub installer_filename: Option<String>,
    pub installer_url: Option<String>,
    pub universal_filename: Option<String>,
    pub universal_url: Option<String>,
    pub changelog_url: Option<String>,
    pub long_version: String,
}

impl ForgeProcessedVersion {
    pub fn new(entry: &ForgeEntry) -> Self {
        let mut ver = Self {
            build: entry.build,
            raw_version: entry.version.clone(),
            mc_version: entry.mc_version.clone(),
            mc_version_sane: entry.mc_version.replacen("_pre", "-pre", 1),
            branch: entry.branch.clone(),
            installer_filename: None,
            installer_url: None,
            universal_filename: None,
            universal_url: None,
            changelog_url: None,
            long_version: format!("{}-{}", entry.mc_version, entry.version),
        };
        if let Some(branch) = &ver.branch {
            ver.long_version += &format!("-{}", branch);
        }

        // to quote Scrumplex: "this comment's whole purpose is to say this: cringe"
        if let Some(files) = &entry.files {
            for (classifier, file) in files {
                let extension = &file.extension;
                let filename = file.filename(&ver.long_version);
                let url = file.url(&ver.long_version);

                if (classifier == "installer") && (extension == "jar") {
                    ver.installer_filename = Some(filename);
                    ver.installer_url = Some(url);
                } else if (classifier == "universal" || classifier == "client")
                    && (extension == "jar" || extension == "zip")
                {
                    ver.universal_filename = Some(filename);
                    ver.universal_url = Some(url);
                } else if (classifier == "changelog") && (extension == "txt") {
                    ver.changelog_url = Some(url);
                }
            }
        }

        ver
    }

    pub fn name(&self) -> String {
        format!("Forge {}", self.build)
    }

    pub fn uses_installer(&self) -> bool {
        !(self.installer_url.is_none() || self.mc_version == "1.5.2")
    }

    pub fn filename(&self) -> Option<String> {
        if self.uses_installer() {
            self.installer_filename.clone()
        } else {
            self.universal_filename.clone()
        }
    }

    pub fn url(&self) -> Option<String> {
        if self.uses_installer() {
            self.installer_url.clone()
        } else {
            self.universal_url.clone()
        }
    }

    pub fn is_supported(&self) -> bool {
        if self.url().is_none() {
            return false;
        }

        let mut version_parts = self.raw_version.split('.');
        let num_version_parts = self.raw_version.split('.').count();
        if num_version_parts < 1 {
            return false;
        }

        let major_version_str = version_parts.next().expect("Missing major version");
        let major_version = major_version_str.parse::<i32>();

        if let Ok(major_version) = major_version {
            if major_version >= 37 {
                return false;
            }
        } else {
            return false;
        }

        true
    }
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

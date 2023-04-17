use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use serde_with::skip_serializing_none;
use std::collections::HashMap;

use crate::models::{GradleSpecifier, MetaVersion};

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangVersionManifest {
    /// The latest version of Minecraft.
    pub latest: MojangVersionManifestLatest,
    /// A list of all versions of Minecraft.
    pub versions: Vec<MojangVersionManifestVersion>,
}

/// The latest version of Minecraft.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangVersionManifestLatest {
    /// The latest release version of Minecraft.
    pub release: String,
    /// The latest snapshot version of Minecraft.
    pub snapshot: String,
}

/// A version of Minecraft.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangVersionManifestVersion {
    /// The ID of the version.
    pub id: String,
    /// The type of the version.
    #[serde(rename = "type")]
    pub version_type: String,
    /// The URL to the version's JSON.
    pub url: String,
    /// The time the version was released.
    pub time: String,
    /// The time the version was last updated.
    pub release_time: String,
    /// Compliance level
    pub compliance_level: i32,
    /// The sha1 hash of the version's JSON.
    pub sha1: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: i32,
    pub total_size: i32,
    pub url: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionDownload {
    pub sha1: String,
    pub size: i32,
    pub url: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(deny_unknown_fields)]
pub struct VersionDownloads {
    pub client: VersionDownload,
    pub server: Option<VersionDownload>,
    pub windows_server: Option<VersionDownload>,
    pub client_mappings: Option<VersionDownload>,
    pub server_mappings: Option<VersionDownload>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JavaVersion {
    pub component: String,
    pub major_version: i32,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibraryDownloadInfo {
    pub path: String,
    pub sha1: String,
    pub size: i32,
    pub url: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibraryClassifiers {
    pub javadoc: Option<VersionLibraryDownloadInfo>,
    #[serde(rename = "natives-linux")]
    pub natives_linux: Option<VersionLibraryDownloadInfo>,
    #[serde(rename = "natives-macos")]
    pub natives_macos: Option<VersionLibraryDownloadInfo>,
    #[serde(rename = "natives-osx")]
    pub natives_osx: Option<VersionLibraryDownloadInfo>,
    #[serde(rename = "natives-windows")]
    pub natives_windows: Option<VersionLibraryDownloadInfo>,
    #[serde(rename = "natives-windows-32")]
    pub natives_windows_32: Option<VersionLibraryDownloadInfo>,
    #[serde(rename = "natives-windows-64")]
    pub natives_windows_64: Option<VersionLibraryDownloadInfo>,
    #[serde(rename = "linux-x86_64")]
    pub linux_x86_64: Option<VersionLibraryDownloadInfo>,
    pub sources: Option<VersionLibraryDownloadInfo>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibraryNatives {
    pub linux: Option<String>,
    pub osx: Option<String>,
    pub windows: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibraryDownloads {
    pub artifact: Option<VersionLibraryDownloadInfo>,
    pub classifiers: Option<VersionLibraryClassifiers>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibraryExtract {
    pub exclude: Vec<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibrary {
    pub name: String,
    pub downloads: VersionLibraryDownloads,
    pub natives: Option<VersionLibraryNatives>,
    pub extract: Option<VersionLibraryExtract>,
    #[validate]
    pub rules: Option<Vec<ManifestRule>>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestRule {
    pub action: String,
    pub os: Option<ManifestRuleOS>,
    #[validate]
    pub features: Option<ManifestRuleFeatures>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct ManifestRuleFeatures {
    pub is_demo_user: Option<bool>,
    pub has_custom_resolution: Option<bool>,
    pub has_quick_plays_support: Option<bool>,
    pub is_quick_play_singleplayer: Option<bool>,
    pub is_quick_play_multiplayer: Option<bool>,
    pub is_quick_play_realms: Option<bool>,
    #[serde(flatten)]
    #[validate(custom(validate_empty_unknown_key_map))]
    pub unknown: HashMap<String, serde_json::Value>,
}

fn validate_empty_unknown_key_map(
    map: &HashMap<String, serde_json::Value>,
) -> Result<(), serde_valid::validation::Error> {
    if !map.is_empty() {
        return Err(serde_valid::validation::Error::Custom(format!(
            "There are unknown keys present: {:?}",
            map
        )));
    }

    Ok(())
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestRuleOS {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLogging {
    pub client: VersionLoggingClient,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLoggingClient {
    pub argument: String,
    pub file: VersionLoggingClientFile,
    #[serde(rename = "type")]
    pub logging_type: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLoggingClientFile {
    pub id: String,
    pub sha1: String,
    pub size: i32,
    pub url: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(untagged)]
pub enum VersionArgument {
    String(String),
    Object(#[validate] VersionArgumentObject),
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(untagged)]
pub enum VersionArgumentValue {
    String(String),
    Array(Vec<String>),
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionArgumentObject {
    #[validate]
    pub rules: Vec<ManifestRule>,
    pub value: VersionArgumentValue,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionArguments {
    #[validate]
    pub game: Vec<VersionArgument>,
    #[validate]
    pub jvm: Vec<VersionArgument>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MinecraftVersion {
    pub asset_index: AssetIndex,
    pub assets: String,
    pub compliance_level: Option<i32>,
    pub downloads: Option<VersionDownloads>,
    pub id: String,
    pub java_version: Option<JavaVersion>,
    #[validate]
    pub libraries: Vec<VersionLibrary>,
    pub logging: Option<VersionLogging>,
    pub main_class: String,
    pub minecraft_arguments: Option<String>,
    #[validate]
    pub arguments: Option<VersionArguments>,
    pub minimum_launcher_version: i32,
    pub release_time: String,
    pub time: String,
    #[serde(rename = "type")]
    pub release_type: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct ExperimentEntry {
    pub id: String,
    pub url: String,
    pub wiki: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct ExperimentIndex {
    pub experiments: Vec<ExperimentEntry>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct OldSnapshotEntry {
    pub id: String,
    pub url: String,
    pub wiki: Option<String>,
    pub jar: String,
    pub sha1: String,
    pub size: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct OldSnapshotIndex {
    pub old_snapshots: Vec<OldSnapshotEntry>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyOverrideEntry {
    main_class: Option<String>,
    applet_class: Option<String>,
    release_time: Option<String>,
    #[serde(rename = "+traits")]
    additional_traits: Option<Vec<String>>,
    #[serde(rename = "+jvmArgs")]
    additional_jvm_args: Option<Vec<String>>,
}

impl LegacyOverrideEntry {
    pub fn apply_onto_meta_version(self, meta_version: &MetaVersion, legacy: bool) {
        // simply hard override classes
        // meta_version.main_class = self.main_class
        // meta_version.applet_class = self.applet_class
        // # if we have an updated release time (more correct than Mojang), use it
        // if self.release_time:
        //     meta_version.release_time = self.release_time

        // # add traits, if any
        // if self.additional_traits:
        //     if not meta_version.additional_traits:
        //         meta_version.additional_traits = []
        //     meta_version.additional_traits += self.additional_traits

        // if self.additional_jvm_args:
        //     if not meta_version.additional_jvm_args:
        //         meta_version.additional_jvm_args = []
        //     meta_version.additional_jvm_args += self.additional_jvm_args

        // if legacy:
        //     # remove all libraries - they are not needed for legacy
        //     meta_version.libraries = None
        //     # remove minecraft arguments - we use our own hardcoded ones
        //     meta_version.minecraft_arguments = None
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct LegacyOverrideIndex {
    versions: HashMap<String, LegacyOverrideEntry>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct LibraryPatch {
    #[serde(rename = "match")]
    pub patch_match: Vec<GradleSpecifier>,
    #[serde(rename = "override")]
    pub patch_override: Option<Library>,
    pub additionalLibraries: Option<Vec<Library>>,
    #[serde(default = "default_library_patch_patch_additional_libraries")]
    pub patchAdditionalLibraries: bool,
}

fn default_library_patch_patch_additional_libraries() -> bool {
    false
}

impl LibraryPatch {
    pub fn applies(&self, target: &Library) -> bool {
        return self.patch_match.contains(target);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct LibraryPatches {
    root: Vec<LibraryPatch>,
}

impl Deref for LibraryPatches {
    type Target = Vec<LibraryPatch>;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct MojangArgumentObject {}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub enum MojangArgument {
    String(String),
    Object(MojangArgumentObject),
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct MojangArguments {
    pub game: Option<Vec<MojangArgument>>, // mixture of strings and objects
    pub jvm: Option<Vec<MojangArgument>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct MojangLoggingArtifact {
    id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct MojangLogging {
    file: MojangLoggingArtifact,
    argument: String,
    #[serde(rename = "type")]
    #[validate(custom(mojang_logging_validate_type))]
    logging_type: String,
}

fn mojang_logging_validate_type(
    logging_type: String,
) -> Result<(), serde_valid::validation::Error> {
    if !vec!["log4j2-xml"].contains(&logging_type) {
        Err(serde_valid::validation::Error::Custom(format!(
            "invalid log type: {}",
            &logging_type
        )))
    } else {
        Ok(())
    }
}

fn default_java_version_component() -> String {
    "jre-legacy".to_string()
}
fn default_java_version_major_version() -> i32 {
    8
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JavaVersion {
    #[serde(default = "default_java_version_component")]
    pub component: String,
    #[serde(default = "default_java_version_major_version")]
    pub major_version: i32,
}

impl Default for JavaVersion {
    fn default() -> Self {
        Self {
            component: default_java_version_component(),
            major_version: default_java_version_major_version(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangVersion {
    pub id: String, // TODO: optional?
    pub arguments: Option<MojangArguments>,
    pub asset_index: Option<MojangAssets>,
    pub assets: Option<String>,
    pub downloads: Option<HashMap<String, MojangArtifactBase>>, // TODO improve this?
    pub libraries: Option<Vec<MojangLibrary>>,                  // TODO: optional?
    pub main_class: Option<String>,
    pub applet_class: Option<String>,
    pub processArguments: Option<String>,
    pub minecraft_arguments: Option<String>,
    pub minimum_launcher_version: Option<i32>,
    pub release_time: Option<String>,
    pub time: Option<String>,
    #[serde(rename = "type")]
    pub version_type: Option<String>,
    pub inherits_from: Option<String>,
    pub logging: Option<HashMap<String, MojangLogging>>, // TODO improve this?
    pub compliance_level: Option<i32>,
    pub javaVersion: Option<JavaVersion>,
}
// class MojangVersion(MetaBase):
//     @validator("minimum_launcher_version")
//     def validate_minimum_launcher_version(cls, v):
//         assert v <= SUPPORTED_LAUNCHER_VERSION
//         return v

//     @validator("compliance_level")
//     def validate_compliance_level(cls, v):
//         assert v <= SUPPORTED_COMPLIANCE_LEVEL
//         return v

//     def to_meta_version(self, name: str, uid: str, version: str) -> MetaVersion:
//         main_jar = None
//         addn_traits = None
//         new_type = self.type
//         compatible_java_majors = None
//         if self.id:
//             client_download = self.downloads["client"]
//             artifact = MojangArtifact(
//                 url=client_download.url,
//                 sha1=client_download.sha1,
//                 size=client_download.size,
//             )
//             downloads = MojangLibraryDownloads(artifact=artifact)
//             main_jar = Library(
//                 name=GradleSpecifier("com.mojang", "minecraft", self.id, "client"),
//                 downloads=downloads,
//             )

//         if not self.compliance_level:  # both == 0 and is None
//             pass
//         elif self.compliance_level == 1:
//             if not addn_traits:
//                 addn_traits = []
//             addn_traits.append("XR:Initial")
//         else:
//             raise Exception(f"Unsupported compliance level {self.compliance_level}")

//         major = DEFAULT_JAVA_MAJOR
//         if (
//             self.javaVersion is not None
//         ):  # some versions don't have this. TODO: maybe maintain manual overrides
//             major = self.javaVersion.major_version

//         compatible_java_majors = [major]
//         if (
//             major in COMPATIBLE_JAVA_MAPPINGS
//         ):  # add more compatible Java versions, e.g. 16 and 17 both work for MC 1.17
//             compatible_java_majors += COMPATIBLE_JAVA_MAPPINGS[major]

//         if new_type == "pending":  # experiments from upstream are type=pending
//             new_type = "experiment"

//         return MetaVersion(
//             name=name,
//             uid=uid,
//             version=version,
//             asset_index=self.asset_index,
//             libraries=self.libraries,
//             main_class=self.main_class,
//             minecraft_arguments=self.minecraft_arguments,
//             release_time=self.release_time,
//             type=new_type,
//             compatible_java_majors=compatible_java_majors,
//             additional_traits=addn_traits,
//             main_jar=main_jar,
//         )

#[cfg(test)]
mod tests {

    use serde_valid::Validate;

    #[test]
    fn test_deserialization() {
        // meta dir is ./meta
        let cwd = std::env::current_dir().unwrap();
        let meta_dir = cwd.join("../meta/mojang");
        println!("meta_dir: {:?}", meta_dir);

        let version_manifest = serde_json::from_str::<super::MojangVersionManifest>(
            &std::fs::read_to_string(meta_dir.join("version_manifest_v2.json")).unwrap(),
        );
        if let Err(e) = version_manifest {
            panic!("Failed to deserialize version manifest: {:?}", e);
        }

        // loop through all files in meta_dir/versions
        for entry in std::fs::read_dir(meta_dir.join("versions")).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let version = serde_json::from_str::<super::MinecraftVersion>(
                    &std::fs::read_to_string(path).unwrap(),
                );
                if let Err(e) = version {
                    panic!(
                        "Failed to deserialize version {}: {:?}",
                        entry.file_name().to_str().unwrap(),
                        e
                    );
                }
                if let Err(e) = version.unwrap().validate() {
                    panic!(
                        "Failed to validate version {}: \n{}\n",
                        entry.file_name().to_str().unwrap(),
                        serde_json::to_string_pretty(&e).unwrap()
                    )
                }
            }
        }
    }
}

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangVersionManifest {
    /// The latest version of Minecraft.
    pub latest: MojangVersionManifestLatest,
    /// A list of all versions of Minecraft.
    pub versions: Vec<MojangVersionManifestVersion>,
}

/// The latest version of Minecraft.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangVersionManifestLatest {
    /// The latest release version of Minecraft.
    pub release: String,
    /// The latest snapshot version of Minecraft.
    pub snapshot: String,
}

/// A version of Minecraft.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
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
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: i32,
    pub total_size: i32,
    pub url: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionDownload {
    pub sha1: String,
    pub size: i32,
    pub url: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VersionDownloads {
    pub client: VersionDownload,
    pub server: Option<VersionDownload>,
    pub windows_server: Option<VersionDownload>,
    pub client_mappings: Option<VersionDownload>,
    pub server_mappings: Option<VersionDownload>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JavaVersion {
    pub component: String,
    pub major_version: i32,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibraryDownloadInfo {
    pub path: String,
    pub sha1: String,
    pub size: i32,
    pub url: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
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
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibraryNatives {
    pub linux: Option<String>,
    pub osx: Option<String>,
    pub windows: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibraryDownloads {
    pub artifact: Option<VersionLibraryDownloadInfo>,
    pub classifiers: Option<VersionLibraryClassifiers>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibraryExtract {
    pub exclude: Vec<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLibrary {
    pub name: String,
    pub downloads: VersionLibraryDownloads,
    pub natives: Option<VersionLibraryNatives>,
    pub extract: Option<VersionLibraryExtract>,
    pub rules: Option<Vec<ManifestRule>>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestRule {
    pub action: String,
    pub os: Option<ManifestRuleOS>,
    pub features: Option<ManifestRuleFeatures>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ManifestRuleFeatures {
    pub is_demo_user: Option<bool>,
    pub has_custom_resolution: Option<bool>,
    pub has_quick_plays_support: Option<bool>,
    pub is_quick_play_singleplayer: Option<bool>,
    pub is_quick_play_multiplayer: Option<bool>,
    pub is_quick_play_realms: Option<bool>,
    // #[serde(flatten)]
    // pub unknown: HashMap<String, serde_json::Value>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestRuleOS {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLogging {
    pub client: VersionLoggingClient,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLoggingClient {
    pub argument: String,
    pub file: VersionLoggingClientFile,
    #[serde(rename = "type")]
    pub logging_type: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLoggingClientFile {
    pub id: String,
    pub sha1: String,
    pub size: i32,
    pub url: String,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum VersionArgument {
    String(String),
    Object(VersionArgumentObject),
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum VersionArgumentValue {
    String(String),
    Array(Vec<String>),
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionArgumentObject {
    pub rules: Vec<ManifestRule>,
    pub value: VersionArgumentValue,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionArguments {
    pub game: Vec<VersionArgument>,
    pub jvm: Vec<VersionArgument>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MinecraftVersion {
    pub asset_index: AssetIndex,
    pub assets: String,
    pub compliance_level: Option<i32>,
    pub downloads: VersionDownloads,
    pub id: String,
    pub java_version: Option<JavaVersion>,
    pub libraries: Vec<VersionLibrary>,
    pub logging: Option<VersionLogging>,
    pub main_class: String,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<VersionArguments>,
    pub minimum_launcher_version: i32,
    pub release_time: String,
    pub time: String,
    #[serde(rename = "type")]
    pub release_type: String,
}

#[cfg(test)]
mod tests {
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
            }
        }
    }
}

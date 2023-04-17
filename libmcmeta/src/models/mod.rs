use core::ops::Deref;
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use std::collections::HashMap;
use std::{fmt::Display, str::FromStr};

pub mod forge;
pub mod mojang;

custom_error! { pub ModelError
    InvalidGradleSpecifier { specifier: String } = "Invalid Gradle specifier '{specifier}'",
}

static META_FORMAT_VERSION: i32 = 1;

/// A Gradle specifier.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct GradleSpecifier {
    /// Group of the artifact.
    pub group: String,
    /// Artifact name.
    pub artifact: String,
    /// Version of the artifact.
    pub version: String,
    /// File extension of the artifact.
    pub extension: Option<String>,
    /// Classifier of the artifact.
    pub classifier: Option<String>,
}

impl GradleSpecifier {
    /// Returns the file name of the artifact.
    pub fn filename(&self) -> String {
        if let Some(classifier) = &self.classifier {
            format!(
                "{}-{}-{}.{}",
                self.artifact,
                self.version,
                classifier,
                self.extension.as_ref().unwrap_or(&"".to_string())
            )
        } else {
            format!(
                "{}-{}.{}",
                self.artifact,
                self.version,
                self.extension.as_ref().unwrap_or(&"".to_string())
            )
        }
    }

    /// Returns the base path of the artifact.
    pub fn base(&self) -> String {
        format!(
            "{}/{}/{}",
            self.group.replace('.', "/"),
            self.artifact,
            self.version
        )
    }

    /// Returns the full path of the artifact.
    pub fn path(&self) -> String {
        format!("{}/{}", self.base(), self.filename())
    }

    /// Returns `true` if the specifier is a LWJGL artifact.
    pub fn is_lwjgl(&self) -> bool {
        vec![
            "org.lwjgl",
            "org.lwjgl.lwjgl",
            "net.java.jinput",
            "net.java.jutils",
        ]
        .contains(&self.group.as_str())
    }

    /// Returns `true` if the specifier is a Log4j artifact.
    pub fn is_log4j(&self) -> bool {
        vec!["org.apache.logging.log4j"].contains(&self.group.as_str())
    }
}

impl FromStr for GradleSpecifier {
    type Err = ModelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let at_split = s.split('@').collect::<Vec<&str>>();

        let components = at_split
            .first()
            .ok_or(ModelError::InvalidGradleSpecifier {
                specifier: s.to_string(),
            })?
            .split(':')
            .collect::<Vec<&str>>();

        let group = components
            .first()
            .ok_or(ModelError::InvalidGradleSpecifier {
                specifier: s.to_string(),
            })?
            .to_string();
        let artifact = components
            .get(1)
            .ok_or(ModelError::InvalidGradleSpecifier {
                specifier: s.to_string(),
            })?
            .to_string();
        let version = components
            .get(2)
            .ok_or(ModelError::InvalidGradleSpecifier {
                specifier: s.to_string(),
            })?
            .to_string();

        let mut extension = Some("jar".to_string());
        if at_split.len() == 2 {
            extension = Some(at_split[1].to_string());
        }

        let classifier = if components.len() == 4 {
            Some(
                components
                    .get(3)
                    .ok_or(ModelError::InvalidGradleSpecifier {
                        specifier: s.to_string(),
                    })?
                    .to_string(),
            )
        } else {
            None
        };

        Ok(GradleSpecifier {
            group,
            artifact,
            version,
            extension,
            classifier,
        })
    }
}

impl Display for GradleSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let extension = if let Some(ext) = &self.extension {
            if ext != "jar" {
                format!("@{}", ext)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if let Some(classifier) = self.classifier.as_ref() {
            write!(
                f,
                "{}:{}:{}:{}{}",
                self.group, self.artifact, self.version, classifier, extension
            )
        } else {
            write!(
                f,
                "{}:{}:{}{}",
                self.group, self.artifact, self.version, extension
            )
        }
    }
}

impl Serialize for GradleSpecifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for GradleSpecifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge)]
#[serde(rename_all = "camelCase")]
pub struct MojangArtifactBase {
    #[merge(strategy = merge::option::overwrite_some)]
    pub sha1: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub size: Option<i32>,
    #[merge(strategy = merge::overwrite)]
    pub url: String,
    #[serde(flatten)]
    #[merge(strategy = merge::hashmap::overwrite_key)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangAssets {
    #[merge(strategy = merge::option::overwrite_some)]
    pub sha1: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub size: Option<i32>,
    #[merge(strategy = merge::overwrite)]
    pub url: String,
    #[merge(strategy = merge::overwrite)]
    pub id: String,
    #[merge(strategy = merge::overwrite)]
    pub total_size: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangArtifact {
    #[merge(strategy = merge::option::overwrite_some)]
    pub sha1: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub size: Option<i32>,
    #[merge(strategy = merge::overwrite)]
    pub url: String,
    #[merge(strategy = merge::option::overwrite_some)]
    pub path: Option<String>,
}

/// ```json
/// "rules": [
///     {
///         "action": "allow"
///     },
///     {
///         "action": "disallow",
///         "os": {
///             "name": "osx"
///         }
///     }
/// ]
/// ```
#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangLibraryExtractRules {
    #[merge(strategy = merge::vec::append)]
    pub exclude: Vec<String>, // TODO maybe drop this completely?
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangLibraryDownloads {
    #[merge(strategy = merge::option::overwrite_some)]
    pub artifact: Option<MojangArtifact>,
    #[merge(strategy = merge::option_hashmap::recurse_some)]
    pub classifiers: Option<HashMap<String, MojangArtifact>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OSRule {
    #[validate(custom(os_rule_name_must_be_os))]
    #[merge(strategy = merge::overwrite)]
    pub name: String,
    #[merge(strategy = merge::option::overwrite_some)]
    pub version: Option<String>,
}

fn os_rule_name_must_be_os(name: &String) -> Result<(), serde_valid::validation::Error> {
    let valid_os_names = vec![
        "osx",
        "linux",
        "windows",
        "windows-arm64",
        "osx-arm64",
        "linux-arm64",
        "linux-arm32",
    ];
    if !valid_os_names.contains(&name.as_str()) {
        Err(serde_valid::validation::Error::Custom(format!(
            "`{}` not a valid os name",
            &name
        )))
    } else {
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangRule {
    #[validate(custom(mojang_rule_action_must_be_allow_disallow))]
    #[merge(strategy = merge::overwrite)]
    pub action: String,
    #[merge(strategy = merge::option::recurse)]
    pub os: Option<OSRule>,
}

fn mojang_rule_action_must_be_allow_disallow(
    action: &String,
) -> Result<(), serde_valid::validation::Error> {
    if !vec!["allow", "disallow"].contains(&action.as_str()) {
        Err(serde_valid::validation::Error::Custom(format!(
            "`{}` not a valid action, must be `allow` or `disallow`",
            &action
        )))
    } else {
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangRules {
    #[merge(strategy = merge::vec::append)]
    root: Vec<MojangRule>,
}

impl Deref for MojangRules {
    type Target = Vec<MojangRule>;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangLibrary {
    #[merge(strategy = merge::option::recurse)]
    pub extract: Option<MojangLibraryExtractRules>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub name: Option<GradleSpecifier>,
    #[merge(strategy = merge::option::recurse)]
    pub downloads: Option<MojangLibraryDownloads>,
    #[merge(strategy = merge::option_hashmap::overwrite_key_some)]
    pub natives: Option<HashMap<String, String>>,
    #[merge(strategy = merge::option::recurse)]
    pub rules: Option<MojangRules>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Library {
    #[merge(strategy = merge::option::recurse)]
    pub extract: Option<MojangLibraryExtractRules>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub name: Option<GradleSpecifier>,
    #[merge(strategy = merge::option::recurse)]
    pub downloads: Option<MojangLibraryDownloads>,
    #[merge(strategy = merge::option_hashmap::overwrite_key_some)]
    pub natives: Option<HashMap<String, String>>,
    #[merge(strategy = merge::option::recurse)]
    pub rules: Option<MojangRules>,
    #[merge(strategy = merge::option::overwrite_some)]
    url: Option<String>,
    #[serde(rename = "MMC-hint")]
    #[merge(strategy = merge::option::overwrite_some)]
    mmc_hint: Option<String>,
}

impl From<MojangLibrary> for Library {
    fn from(item: MojangLibrary) -> Self {
        Self {
            extract: item.extract,
            name: item.name,
            downloads: item.downloads,
            natives: item.natives,
            rules: item.rules,
            url: None,
            mmc_hint: None,
        }
    }
}

impl From<Library> for MojangLibrary {
    fn from(item: Library) -> Self {
        Self {
            extract: item.extract,
            name: item.name,
            downloads: item.downloads,
            natives: item.natives,
            rules: item.rules,
        }
    }
}

impl From<&MojangLibrary> for Library {
    fn from(item: &MojangLibrary) -> Self {
        Self {
            extract: item.extract.clone(),
            name: item.name.clone(),
            downloads: item.downloads.clone(),
            natives: item.natives.clone(),
            rules: item.rules.clone(),
            url: None,
            mmc_hint: None,
        }
    }
}

impl From<&Library> for MojangLibrary {
    fn from(item: &Library) -> Self {
        Self {
            extract: item.extract.clone(),
            name: item.name.clone(),
            downloads: item.downloads.clone(),
            natives: item.natives.clone(),
            rules: item.rules.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Dependency {
    #[merge(strategy = merge::overwrite)]
    pub uid: String,
    #[merge(strategy = merge::option::overwrite_some)]
    pub equals: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub suggests: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetaVersion {
    #[merge(strategy = merge::overwrite)]
    pub format_version: i32,
    #[merge(strategy = merge::overwrite)]
    pub name: String,
    #[merge(strategy = merge::overwrite)]
    pub version: String,
    #[merge(strategy = merge::overwrite)]
    pub uid: String,
    #[serde(rename = "type")]
    #[merge(strategy = merge::option::overwrite_some)]
    pub version_type: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub order: Option<i32>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub volatile: Option<bool>,
    #[merge(strategy = merge::option_vec::append_some)]
    pub requires: Option<Vec<Dependency>>,
    #[merge(strategy = merge::option_vec::append_some)]
    pub conflicts: Option<Vec<Dependency>>,
    #[merge(strategy = merge::option_vec::append_some)]
    pub libraries: Option<Vec<Library>>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub asset_index: Option<MojangAssets>,
    #[merge(strategy = merge::option_vec::append_some)]
    pub maven_files: Option<Vec<Library>>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub main_jar: Option<Library>,
    #[merge(strategy = merge::option_vec::append_some)]
    pub jar_mods: Option<Vec<Library>>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub main_class: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub applet_class: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub minecraft_arguments: Option<String>,
    #[merge(strategy = merge::option::overwrite_some)]
    pub release_time: Option<String>,
    #[merge(strategy = merge::option_vec::append_some)]
    pub compatible_java_majors: Option<Vec<i32>>,
    #[merge(strategy = merge::option_vec::append_some)]
    pub additional_traits: Option<Vec<String>>,
    #[serde(rename = "+tweakers")]
    #[merge(strategy = merge::option_vec::append_some)]
    pub additional_tweakers: Option<Vec<String>>,
    #[serde(rename = "+jvmArgs")]
    #[merge(strategy = merge::option_vec::append_some)]
    pub additional_jvm_args: Option<Vec<String>>,
}

pub mod validation {
    pub fn is_some<T>(obj: Option<T>) -> Result<(), serde_valid::validation::Error> {
        if !obj.is_some() {
            return Err(serde_valid::validation::Error::Custom(
                "Must be some".to_string(),
            ));
        }
        Ok(())
    }
}

pub mod merge {
    pub use merge::Merge;
    pub use merge::{bool, num, ord, vec};

    /// generic overwrite stratagy
    pub fn overwrite<T>(left: &mut T, right: T) {
        *left = right
    }

    /// Merge strategies for `Option`
    pub mod option {
        /// Overwrite `left` with `right` only if `left` is `None`.
        pub fn overwrite_none<T>(left: &mut Option<T>, right: Option<T>) {
            if left.is_none() {
                *left = right;
            }
        }

        /// Overwrite `left` with `right` only if `right` is `Some`
        pub fn overwrite_some<T>(left: &mut Option<T>, right: Option<T>) {
            if let Some(new) = right {
                *left = Some(new);
            }
        }

        /// If both `left` and `right` are `Some`, recursively merge the two.
        /// Otherwise, fall back to `overwrite_none`.
        pub fn recurse<T: merge::Merge>(left: &mut Option<T>, right: Option<T>) {
            if let Some(new) = right {
                if let Some(original) = left {
                    original.merge(new);
                } else {
                    *left = Some(new);
                }
            }
        }
    }

    /// Merge strategies for `HashMap`
    pub mod hashmap {
        use std::collections::HashMap;
        use std::hash::Hash;

        pub fn recurse<K: Eq + Hash, V: merge::Merge>(
            left: &mut HashMap<K, V>,
            right: HashMap<K, V>,
        ) {
            use std::collections::hash_map::Entry;

            for (k, v) in right {
                match left.entry(k) {
                    Entry::Occupied(mut existing) => existing.get_mut().merge(v),
                    Entry::Vacant(empty) => {
                        empty.insert(v);
                    }
                }
            }
        }

        pub fn overwrite_key<K: Eq + Hash, V>(left: &mut HashMap<K, V>, right: HashMap<K, V>) {
            use std::collections::hash_map::Entry;

            for (k, v) in right {
                left.insert(k, v);
            }
        }
    }

    /// Merge strategies for `Option<HashMap>`
    pub mod option_hashmap {
        use super::hashmap;
        use std::collections::HashMap;
        use std::hash::Hash;

        pub fn recurse_some<K: Eq + Hash, V: merge::Merge>(
            left: &mut Option<HashMap<K, V>>,
            right: Option<HashMap<K, V>>,
        ) {
            if let Some(new) = right {
                if let Some(original) = left {
                    hashmap::recurse(original, new);
                } else {
                    *left = Some(new);
                }
            }
        }

        pub fn overwrite_key_some<K: Eq + Hash, V>(
            left: &mut Option<HashMap<K, V>>,
            right: Option<HashMap<K, V>>,
        ) {
            use std::collections::hash_map::Entry;
            if let Some(new) = right {
                if let Some(original) = left {
                    hashmap::overwrite_key(original, new);
                } else {
                    *left = Some(new);
                }
            }
        }
    }

    /// Merge strategies for `Option<Vec>`
    pub mod option_vec {

        /// Append the contents of `right` to `left` if `left` and `right` are `Some`
        /// replace the option if `left` is `None` and `right` is `Some`
        pub fn append_some<T>(left: &mut Option<Vec<T>>, right: Option<Vec<T>>) {
            if let Some(mut new) = right {
                if let Some(original) = left {
                    original.append(&mut new);
                } else {
                    *left = Some(new);
                }
            }
        }
    }
}

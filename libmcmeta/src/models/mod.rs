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

/// A Gradle specifier.
#[derive(Debug, PartialEq, Eq, Clone, merge::Merge, Default)]
pub struct GradleSpecifier {
    /// Group of the artifact.
    #[merge(strategy = merge::overwrite)]
    pub group: String,
    /// Artifact name.
    #[merge(strategy = merge::overwrite)]
    pub artifact: String,
    /// Version of the artifact.
    #[merge(strategy = merge::overwrite)]
    pub version: String,
    /// File extension of the artifact.
    #[merge(strategy = merge::overwrite)]
    pub extension: Option<String>,
    /// Classifier of the artifact.
    #[merge(strategy = merge::overwrite)]
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
    pub sha1: Option<String>,
    pub size: Option<i32>,
    pub url: String,
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangAssets {
    pub sha1: Option<String>,
    pub size: Option<i32>,
    pub url: String,
    pub id: String,
    pub total_size: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MojangArtifact {
    pub sha1: Option<String>,
    pub size: Option<i32>,
    pub url: String,
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

fn os_rule_name_must_be_os(name: String) -> Result<(), serde_valid::validation::Error> {
    let valid_os_names = vec![
        "osx",
        "linux",
        "windows",
        "windows-arm64",
        "osx-arm64",
        "linux-arm64",
        "linux-arm32",
    ];
    if !valid_os_names.contains(&name) {
        Err(format!("`{}` not a valid os name", &name))
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
    action: String,
) -> Result<(), serde_valid::validation::Error> {
    if !vec!["allow", "disallow"].contains(&action) {
        Err(format!(
            "`{}` not a valid action, must be `allow` or `disallow`",
            &action
        ))
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
    #[merge(strategy = merge::option::recurse)]
    pub name: Option<GradleSpecifier>,
    #[merge(strategy = merge::option::recurse)]
    pub downloads: Option<MojangLibraryDownloads>,
    #[merge(strategy = merge::option::recurse)]
    pub natives: Option<HashMap<String, String>>,
    #[merge(strategy = merge::option::recurse)]
    pub rules: Option<MojangRules>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Validate, merge::Merge, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Library {
    #[merge(strategy = merge::option::recurse)]
    pub extract: Option<MojangLibraryExtractRules>,
    #[merge(strategy = merge::option::recurse)]
    pub name: Option<GradleSpecifier>,
    #[merge(strategy = merge::option::recurse)]
    pub downloads: Option<MojangLibraryDownloads>,
    #[merge(strategy = merge::option::recurse)]
    pub natives: Option<HashMap<String, String>>,
    #[merge(strategy = merge::option::recurse)]
    pub rules: Option<MojangRules>,
    #[merge(strategy = merge::option::overwrite_some)]
    url: Option<String>,
    #[serde(rename = "MMC-hint")]
    #[merge(strategy = merge::option::overwrite_some)]
    mmcHint: Option<String>,
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
    pub format_version: i32,
    pub name: String,
    pub version: String,
    pub uid: String,
    #[serde(rename = "type")]
    pub version_type: Option<String>,
    pub order: Option<i32>,
    pub volatile: Option<bool>,
    pub requires: Option<Vec<Dependency>>,
    pub conflicts: Option<Vec<Dependency>>,
    pub libraries: Option<Vec<Library>>,
    pub asset_index: Option<MojangAssets>,
    pub maven_files: Option<Vec<Library>>,
    pub main_jar: Option<Library>,
    pub jar_mods: Option<Vec<Library>>,
    pub main_class: Option<String>,
    pub applet_class: Option<String>,
    pub minecraft_arguments: Option<String>,
    pub release_time: Option<String>,
    pub compatible_java_majors: Option<Vec<i32>>,
    pub additional_traits: Option<Vec<String>>,
    #[serde(rename = "+tweakers")]
    pub additional_tweakers: Option<Vec<String>>,
    #[serde(rename = "+jvmArgs")]
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

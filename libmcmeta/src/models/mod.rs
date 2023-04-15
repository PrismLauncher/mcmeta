use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

pub mod forge;
pub mod mojang;

custom_error! { pub ModelError
    InvalidGradleSpecifier { specifier: String } = "Invalid Gradle specifier '{specifier}'",
}

/// A Gradle specifier.
#[derive(Debug, PartialEq, Eq, Clone)]
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

        pub fn recurse_some<K: Eq + Hash, V: merge::Merge>(
            left: &mut Option<HashMap<K, V>>,
            right: Option<HashMap<K, V>>,
        ) {
            if let Some(new) = right {
                if let Some(original) = left {
                    recurse(original, new);
                } else {
                    *left = Some(new);
                }
            }
        }
    }
}

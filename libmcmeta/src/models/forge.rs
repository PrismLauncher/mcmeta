use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ForgeMavenMetadata {
    #[serde(flatten)]
    pub versions: HashMap<String, Vec<String>>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ForgeMavenPromotions {
    pub homepage: String,
    pub promos: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ForgeVersionMeta {
    pub classifiers: ForgeVersionClassifiers,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ForgeVersionClassifier {
    pub txt: Option<String>,
    pub zip: Option<String>,
    pub jar: Option<String>,
    pub stash: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
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
                    panic!("Failed to deserialize version {}: {:?}", forge_version, e);
                }
            }
        }
    }
}

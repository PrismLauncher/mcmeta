use std::sync::Arc;

use axum::{extract::Path, response::IntoResponse, Extension};

use libmcmeta::models::forge::{
    ForgeInstallerManifestVersion, ForgeMavenMetadata, ForgeMavenPromotions, ForgeVersion,
    ForgeVersionMeta,
};

use crate::app_config::{ServerConfig, StorageFormat};
use crate::routes::APIResponse;

pub async fn raw_forge_maven_meta(config: Extension<Arc<ServerConfig>>) -> impl IntoResponse {
    match &config.storage_format {
        StorageFormat::Json { meta_directory } => {
            let metadata_dir = std::path::Path::new(meta_directory);
            let forge_meta_dir = metadata_dir.join("forge");
            let maven_meta_file = forge_meta_dir.join("maven-metadata.json");
            let manifest = serde_json::from_str::<ForgeMavenMetadata>(
                &std::fs::read_to_string(&maven_meta_file).unwrap(),
            )
            .unwrap();

            (
                axum::http::StatusCode::OK,
                axum::Json(APIResponse {
                    data: Some(manifest),
                    error: None,
                }),
            )
        }
        StorageFormat::Database => todo!(),
    }
}

pub async fn raw_forge_promotions(config: Extension<Arc<ServerConfig>>) -> impl IntoResponse {
    match &config.storage_format {
        StorageFormat::Json { meta_directory } => {
            let metadata_dir = std::path::Path::new(meta_directory);
            let forge_meta_dir = metadata_dir.join("forge");
            let promotions_file = forge_meta_dir.join("promotions_slim.json");
            let manifest = serde_json::from_str::<ForgeMavenPromotions>(
                &std::fs::read_to_string(&promotions_file).unwrap(),
            )
            .unwrap();

            (
                axum::http::StatusCode::OK,
                axum::Json(APIResponse {
                    data: Some(manifest),
                    error: None,
                }),
            )
        }
        StorageFormat::Database => todo!(),
    }
}

pub async fn raw_forge_version(
    config: Extension<Arc<ServerConfig>>,
    Path(version): Path<String>,
) -> impl IntoResponse {
    match &config.storage_format {
        StorageFormat::Json { meta_directory } => {
            let metadata_dir = std::path::Path::new(meta_directory);
            let forge_meta_dir = metadata_dir.join("forge");
            let versions_dir = forge_meta_dir.join("version_manifests");
            let version_file = versions_dir.join(format!("{}.json", version));
            if !version_file.exists() {
                return (
                    axum::http::StatusCode::NOT_FOUND,
                    axum::Json(APIResponse {
                        data: None,
                        error: Some(format!("Version {} does not exist", version)),
                    }),
                );
            }
            let manifest = serde_json::from_str::<ForgeVersion>(
                &std::fs::read_to_string(&version_file).unwrap(),
            )
            .unwrap();

            (
                axum::http::StatusCode::OK,
                axum::Json(APIResponse {
                    data: Some(manifest),
                    error: None,
                }),
            )
        }
        StorageFormat::Database => todo!(),
    }
}

pub async fn raw_forge_version_meta(
    config: Extension<Arc<ServerConfig>>,
    Path(version): Path<String>,
) -> impl IntoResponse {
    match &config.storage_format {
        StorageFormat::Json { meta_directory } => {
            let metadata_dir = std::path::Path::new(meta_directory);
            let forge_meta_dir = metadata_dir.join("forge");
            let versions_dir = forge_meta_dir.join("files_manifests");
            let version_file = versions_dir.join(format!("{}.json", version));
            if !version_file.exists() {
                return (
                    axum::http::StatusCode::NOT_FOUND,
                    axum::Json(APIResponse {
                        data: None,
                        error: Some(format!("Version {} does not exist", version)),
                    }),
                );
            }
            let manifest = serde_json::from_str::<ForgeVersionMeta>(
                &std::fs::read_to_string(&version_file).unwrap(),
            )
            .unwrap();

            (
                axum::http::StatusCode::OK,
                axum::Json(APIResponse {
                    data: Some(manifest),
                    error: None,
                }),
            )
        }
        StorageFormat::Database => todo!(),
    }
}

pub async fn raw_forge_version_installer(
    config: Extension<Arc<ServerConfig>>,
    Path(version): Path<String>,
) -> impl IntoResponse {
    match &config.storage_format {
        StorageFormat::Json { meta_directory } => {
            let metadata_dir = std::path::Path::new(meta_directory);
            let forge_meta_dir = metadata_dir.join("forge");
            let versions_dir = forge_meta_dir.join("installer_manifests");
            let version_file = versions_dir.join(format!("{}.json", version));
            if !version_file.exists() {
                return (
                    axum::http::StatusCode::NOT_FOUND,
                    axum::Json(APIResponse {
                        data: None,
                        error: Some(format!("Version {} does not exist", version)),
                    }),
                );
            }
            let manifest = serde_json::from_str::<ForgeInstallerManifestVersion>(
                &std::fs::read_to_string(&version_file).unwrap(),
            )
            .unwrap();

            (
                axum::http::StatusCode::OK,
                axum::Json(APIResponse {
                    data: Some(manifest),
                    error: None,
                }),
            )
        }
        StorageFormat::Database => todo!(),
    }
}

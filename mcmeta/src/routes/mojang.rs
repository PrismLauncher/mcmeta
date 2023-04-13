use axum::{extract::Path, response::IntoResponse, Extension};
use libmcmeta::models::mojang::{MinecraftVersion, MojangVersionManifest};
use std::{path::Path as StdPath, str::FromStr, sync::Arc};

use crate::app_config::{ServerConfig, StorageFormat};
use crate::routes::APIResponse;

pub async fn raw_mojang_manifest(config: Extension<Arc<ServerConfig>>) -> impl IntoResponse {
    match &config.storage_format {
        StorageFormat::Json { meta_directory } => {
            let metadata_dir = std::path::Path::new(meta_directory);
            let mojang_meta_dir = metadata_dir.join("mojang");
            let local_manifest = mojang_meta_dir.join("version_manifest_v2.json");
            let manifest = serde_json::from_str::<MojangVersionManifest>(
                &std::fs::read_to_string(&local_manifest).unwrap(),
            )
            .unwrap();

            axum::Json(APIResponse {
                data: Some(manifest),
                error: None,
            })
        }
        StorageFormat::Database => todo!(),
    }
}

pub async fn raw_mojang_version(
    config: Extension<Arc<ServerConfig>>,
    Path(version): Path<String>,
) -> impl IntoResponse {
    match &config.storage_format {
        StorageFormat::Json { meta_directory } => {
            let metadata_dir = std::path::Path::new(meta_directory);
            let mojang_meta_dir = metadata_dir.join("mojang");
            let versions_dir = mojang_meta_dir.join("versions");
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
            let manifest = serde_json::from_str::<MinecraftVersion>(
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

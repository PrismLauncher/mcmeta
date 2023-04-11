use std::{path::Path as StdPath, sync::Arc};

use app_config::{ServerConfig, StorageFormat};
use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use custom_error::custom_error;
use libmcmeta::models::{
    forge::{
        ForgeInstallerManifestVersion, ForgeMavenMetadata, ForgeMavenPromotions, ForgeVersion,
        ForgeVersionMeta,
    },
    mojang::{MinecraftVersion, MojangVersionManifest},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, info};

mod app_config;
mod download;
mod storage;

custom_error! {pub MetaMCError
    MojangMetadata { source: download::mojang::MojangMetadataError } = "Error while downloading Mojang metadata: {source}",
    Config { source: config::ConfigError } = "Error while reading config from environment",
    Parse { source: std::net::AddrParseError } = "Error while parsing address: {source}",
    Hyper { source: hyper::Error } = "Error while running Hyper: {source}",
    IO { source: std::io::Error } = "Error while reading or writing: {source}",
    Json { source: serde_json::Error } = "Error while serializing or deserializing JSON: {source}",
    Join { source: tokio::task::JoinError } = "Thread join error: {source}",
}

#[derive(Serialize, Debug, Clone)]
pub struct APIResponse<T> {
    pub data: Option<T>,
    pub error: Option<String>,
}

async fn raw_mojang_manifest(config: Extension<Arc<ServerConfig>>) -> impl IntoResponse {
    match &config.storage_format {
        app_config::StorageFormat::Json { meta_directory } => {
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
        app_config::StorageFormat::Database => todo!(),
    }
}

async fn raw_mojang_version(
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

async fn raw_forge_maven_meta(config: Extension<Arc<ServerConfig>>) -> impl IntoResponse {
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

async fn raw_forge_promotions(config: Extension<Arc<ServerConfig>>) -> impl IntoResponse {
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

async fn raw_forge_version(
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

async fn raw_forge_version_meta(
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

async fn raw_forge_version_installer(
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

use tracing_subscriber::{filter, prelude::*};

#[tokio::main]
async fn main() -> Result<(), MetaMCError> {
    let file_appender = tracing_appender::rolling::hourly("logs", "mcmeta.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);
    let stdout_log = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .compact();

    let debug_log = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file)
        .with_filter(filter::LevelFilter::DEBUG);

    tracing_subscriber::registry()
        .with(
            stdout_log
                .with_filter(filter::LevelFilter::INFO)
                .and_then(debug_log),
        )
        .init();

    let config = Arc::new(ServerConfig::from_config()?);
    debug!("Config: {:#?}", config);

    config.storage_format.initialize_metadata().await?;

    let raw_mojang_routes = Router::new()
        .route("/", get(raw_mojang_manifest))
        .route("/:version", get(raw_mojang_version));
    let raw_forge_routes = Router::new()
        .route("/", get(raw_forge_maven_meta))
        .route("/promotions", get(raw_forge_promotions))
        .route("/:version", get(raw_forge_version))
        .route("/:version/meta", get(raw_forge_version_meta))
        .route("/:version/installer", get(raw_forge_version_installer));

    let raw_routes = Router::new()
        .nest("/mojang", raw_mojang_routes)
        .nest("/forge", raw_forge_routes);

    let http = Router::new()
        .nest("/raw", raw_routes)
        .layer(Extension(config.clone()));

    let addr = config.bind_address.parse()?;
    info!("Starting server on {}", addr);
    axum::Server::bind(&addr)
        .serve(http.into_make_service())
        .await?;

    Ok(())
}

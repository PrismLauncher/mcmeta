use std::{path::Path as StdPath, sync::Arc};

use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use custom_error::custom_error;
use libmcmeta::models::mojang::MojangVersionManifest;
use serde::Deserialize;
use serde_json::json;
use tracing::info;

mod download;

custom_error! {pub MetaMCError
    MojangMetadata { source: download::mojang::MojangMetadataError } = "Error while downloading Mojang metadata: {source}",
    Config { source: config::ConfigError } = "Error while reading config from environment",
    Parse { source: std::net::AddrParseError } = "Error while parsing address: {source}",
    Hyper { source: hyper::Error } = "Error while running Hyper: {source}",
    IO { source: std::io::Error } = "Error while reading or writing: {source}",
    Json { source: serde_json::Error } = "Error while serializing or deserializing JSON: {source}",
}

fn default_bind_address() -> String {
    "127.0.0.1:8080".to_string()
}

fn default_meta_directory() -> String {
    "meta".to_string()
}

#[derive(Deserialize, Debug)]
struct ServerConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_meta_directory")]
    pub meta_directory: String,
}

impl ServerConfig {
    fn from_config() -> Result<Self, MetaMCError> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("MCMETA"))
            .build()?;

        config.try_deserialize::<'_, Self>().map_err(Into::into)
    }
}

async fn initialize_mojang_metadata(metadata_dir: &StdPath) -> Result<(), MetaMCError> {
    info!("Checking for Mojang metadata");
    let mojang_meta_dir = metadata_dir.join("mojang");

    if !mojang_meta_dir.exists() {
        info!(
            "Mojang metadata directory at {} does not exist, creating it",
            mojang_meta_dir.display()
        );
        std::fs::create_dir_all(&mojang_meta_dir)?;
    }

    let local_manifest = mojang_meta_dir.join("version_manifest_v2.json");
    if !local_manifest.exists() {
        info!("Mojang metadata does not exist, downloading it");
        let manifest = download::mojang::load_manifest().await?;
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(&local_manifest, manifest_json)?;
    }
    let manifest =
        serde_json::from_str::<MojangVersionManifest>(&std::fs::read_to_string(&local_manifest)?)?;
    let versions_dir = mojang_meta_dir.join("versions");
    if !versions_dir.exists() {
        info!(
            "Mojang versions directory at {} does not exist, creating it",
            versions_dir.display()
        );
        std::fs::create_dir_all(&versions_dir)?;
    }
    for version in &manifest.versions {
        let version_file = versions_dir.join(format!("{}.json", &version.id));
        if !version_file.exists() {
            info!(
                "Mojang metadata for version {} does not exist, downloading it",
                &version.id
            );
            let version_manifest = download::mojang::load_version_manifest(&version.url).await?;
            let version_manifest_json = serde_json::to_string_pretty(&version_manifest)?;
            std::fs::write(&version_file, version_manifest_json)?;
        }
    }

    Ok(())
}

async fn initialize_metadata(config: &ServerConfig) -> Result<(), MetaMCError> {
    // check if metadata directory exists
    let metadata_dir = std::path::Path::new(&config.meta_directory);
    if !metadata_dir.exists() {
        info!(
            "Metadata directory at {} does not exist, creating it",
            &config.meta_directory
        );
        std::fs::create_dir_all(metadata_dir)?;
    }

    initialize_mojang_metadata(metadata_dir).await?;

    Ok(())
}

async fn raw_mojang_manifest(config: Extension<Arc<ServerConfig>>) -> impl IntoResponse {
    let metadata_dir = std::path::Path::new(&config.meta_directory);
    let mojang_meta_dir = metadata_dir.join("mojang");
    let local_manifest = mojang_meta_dir.join("version_manifest_v2.json");
    let manifest = serde_json::from_str::<MojangVersionManifest>(
        &std::fs::read_to_string(&local_manifest).unwrap(),
    )
    .unwrap();

    axum::Json(manifest)
}

async fn raw_mojang_version(
    config: Extension<Arc<ServerConfig>>,
    Path(version): Path<String>,
) -> impl IntoResponse {
    let metadata_dir = std::path::Path::new(&config.meta_directory);
    let mojang_meta_dir = metadata_dir.join("mojang");
    let versions_dir = mojang_meta_dir.join("versions");
    let version_file = versions_dir.join(format!("{}.json", version));
    if !version_file.exists() {
        return (
            axum::http::StatusCode::NOT_FOUND,
            axum::Json(json!("Version not found")),
        );
    }
    let manifest =
        serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&version_file).unwrap())
            .unwrap();

    (axum::http::StatusCode::OK, axum::Json(manifest))
}

#[tokio::main]
async fn main() -> Result<(), MetaMCError> {
    tracing_subscriber::fmt::init();
    let config = Arc::new(ServerConfig::from_config()?);

    initialize_metadata(&config).await?;

    let raw_mojang_routes = Router::new()
        .route("/", get(raw_mojang_manifest))
        .route("/:version", get(raw_mojang_version));

    let raw_routes = Router::new().nest("/mojang", raw_mojang_routes);

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

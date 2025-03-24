use std::sync::Arc;

use axum::extract::State;
use axum::{extract::Path, response::IntoResponse};

use libmcmeta::models::forge::{
    ForgeInstallerManifestVersion, ForgeMavenMetadata, ForgeMavenPromotions, ForgeVersion,
    ForgeVersionMeta,
};
use tracing::instrument;

use crate::storage::Storage;

use super::{into_api_axum_responce, ServerState};

#[instrument]
pub async fn raw_forge_maven_meta(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let result = state
        .upstream_storage
        .fetch_record::<ForgeMavenMetadata>(["forge"], "maven-metadata")
        .await;
    into_api_axum_responce(result, "sorge Maven metadata not found")
}

#[instrument]
pub async fn raw_forge_promotions(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let result = state
        .upstream_storage
        .fetch_record::<ForgeMavenPromotions>(["forge"], "promotions_slim")
        .await;
    into_api_axum_responce(result, "Forge Maven Promotions data not found")
}

#[instrument]
pub async fn raw_forge_version(
    State(state): State<Arc<ServerState>>,
    Path(version): Path<String>,
) -> impl IntoResponse {
    let result = state
        .upstream_storage
        .fetch_record::<ForgeVersion>(["forge", "version_manifests"], &version)
        .await;
    into_api_axum_responce(result, format!("Version {} does not exist", version))
}

#[instrument]
pub async fn raw_forge_version_meta(
    State(state): State<Arc<ServerState>>,
    Path(version): Path<String>,
) -> impl IntoResponse {
    let result = state
        .upstream_storage
        .fetch_record::<ForgeVersionMeta>(["forge", "files_manifests"], &version)
        .await;
    into_api_axum_responce(result, format!("Version {} does not exist", version))
}

#[instrument]
pub async fn raw_forge_version_installer(
    State(state): State<Arc<ServerState>>,
    Path(version): Path<String>,
) -> impl IntoResponse {
    let result = state
        .upstream_storage
        .fetch_record::<ForgeInstallerManifestVersion>(["forge", "installer_manifests"], &version)
        .await;
    into_api_axum_responce(result, format!("Version {} does not exist", version))
}

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use libmcmeta::models::mojang::{MinecraftVersion, MojangVersionManifest};
use std::sync::Arc;
use tracing::instrument;

use crate::storage::Storage;

use super::{into_api_axum_responce, ServerState};

#[instrument]
pub async fn raw_mojang_manifest(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let manifest = state
        .upstream_storage
        .fetch_record::<MojangVersionManifest>(["mojang"], "version_manifest_v2")
        .await;
    into_api_axum_responce(manifest, "Version manifest not found")
}

#[instrument]
pub async fn raw_mojang_version(
    State(state): State<Arc<ServerState>>,
    Path(version): Path<String>,
) -> impl IntoResponse {
    let result = state
        .upstream_storage
        .fetch_record::<MinecraftVersion>(["mojang", "versions"], &version)
        .await;
    into_api_axum_responce(result, format!("Version {} does not exits", &version))
}

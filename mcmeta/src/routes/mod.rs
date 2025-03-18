use std::sync::Arc;

use serde::Serialize;

use crate::{app_config::MetaConfig, storage::StorageImpl};

pub mod forge;
pub mod mojang;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ServerState {
    pub config: Arc<MetaConfig>,
    pub upstream_storage: Arc<StorageImpl>,
    pub generated_storage: Arc<StorageImpl>,
}

#[derive(Serialize, Debug, Clone)]
pub struct APIResponse<T> {
    pub data: Option<T>,
    pub error: Option<String>,
}

pub fn into_api_axum_responce<T, E>(
    result: Result<Option<T>, E>,
    not_found: impl Into<String>,
) -> (axum::http::StatusCode, axum::Json<APIResponse<T>>)
where
    T: Serialize,
    E: AsRef<dyn core::error::Error>,
{
    match result {
        Ok(Some(t)) => (
            axum::http::StatusCode::OK,
            axum::Json(APIResponse::from_some(t)),
        ),
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            axum::Json(APIResponse::from_err(not_found)),
        ),
        Err(err) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(APIResponse::from_err(err.as_ref().to_string())),
        ),
    }
}

impl<T> APIResponse<T> {
    pub fn from_some(value: T) -> Self
    where
        T: Serialize,
    {
        APIResponse {
            data: Some(value),
            error: None,
        }
    }

    pub fn from_err(err: impl Into<String>) -> Self {
        APIResponse {
            data: None,
            error: Some(err.into()),
        }
    }

    #[allow(dead_code)]
    pub fn from_result<E>(result: Result<T, E>) -> Self
    where
        T: Serialize,
        E: AsRef<dyn core::error::Error>,
    {
        match result {
            Ok(d) => APIResponse {
                data: Some(d),
                error: None,
            },
            Err(e) => APIResponse {
                data: None,
                error: Some(e.as_ref().to_string()),
            },
        }
    }
}

use serde::Serialize;

pub mod forge;
pub mod mojang;

#[derive(Serialize, Debug, Clone)]
pub struct APIResponse<T> {
    pub data: Option<T>,
    pub error: Option<String>,
}

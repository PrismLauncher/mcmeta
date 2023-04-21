use crate::utils::get_json_context_back;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("Unable to deserialise json object at {line}:{column}. Context `{ctx}` \n\nCaused by:\n\t{source}")]
    BadJsonData {
        ctx: String,
        line: usize,
        column: usize,
        source: serde_json::Error,
    },
}

impl MetadataError {
    pub fn from_json_err(err: serde_json::Error, body: &str) -> Self {
        Self::BadJsonData {
            ctx: get_json_context_back(&err, body, 200),
            line: err.line(),
            column: err.column(),
            source: err,
        }
    }
}

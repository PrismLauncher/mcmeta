use crate::utils::{get_json_context, get_json_context_back};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error(
        "Unable to deserialise json object at {line}:{column}. \n\tContext: `{ctx}` \n\nCaused by:\n\t{source}"
    )]
    BadJsonData {
        ctx: String,
        line: usize,
        column: usize,
        source: serde_json::Error,
    },
    #[error("Unable to find version manifest in {0}")]
    MissingMojangVersionManifest(String),
    #[error("Errors during {0}: {1:?}")]
    BulkProcessingError(String, Vec<eyre::Report>),
}

impl MetadataError {
    pub fn from_json_err(err: serde_json::Error, body: &str) -> Self {
        let mut ctx = get_json_context_back(&err, body, 200);
        ctx.push_str(&get_json_context(&err, body, 200));

        Self::BadJsonData {
            ctx,
            line: err.line(),
            column: err.column(),
            source: err,
        }
    }
}

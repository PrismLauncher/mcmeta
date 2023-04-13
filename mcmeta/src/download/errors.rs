use crate::utils::get_json_context_back;
use custom_error::custom_error;

custom_error! {
    pub MetadataError
    Config { source: config::ConfigError } = "Error while reading config from environment",
    Request { source: reqwest::Error } = "Request error: {source}",
    Deserialization { source: serde_json::Error } = "Deserialization error: {source}",
    BadData {
        ctx: String,
        source: serde_json::Error
    } = @{
        format!("{}. Context at {}:{} (may be truncated) \" {} \"", source, source.line(), source.column(), ctx)
    },
    Validation { source: serde_valid::validation::Errors } = "Validation Error: {source}",
}

impl MetadataError {
    pub fn from_json_err(err: serde_json::Error, body: &str) -> Self {
        match err.classify() {
            serde_json::error::Category::Data => Self::BadData {
                ctx: get_json_context_back(&err, body, 200),
                source: err,
            },
            _ => err.into(),
        }
    }
}

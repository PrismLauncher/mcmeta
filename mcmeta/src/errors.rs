use crate::download::errors::MetadataError;
use custom_error::custom_error;

custom_error! {
    pub MetaMCError
    MojangMetadata { source: MetadataError } = "Error while downloading metadata: {source}",
    Config { source: config::ConfigError } = "Error while reading config from environment",
    Parse { source: std::net::AddrParseError } = "Error while parsing address: {source}",
    Hyper { source: hyper::Error } = "Error while running Hyper: {source}",
    IO { source: std::io::Error } = "Error while reading or writing: {source}",
    Json { source: serde_json::Error } = "Error while serializing or deserializing JSON: {source}",
    Join { source: tokio::task::JoinError } = "Thread join error: {source}",
}

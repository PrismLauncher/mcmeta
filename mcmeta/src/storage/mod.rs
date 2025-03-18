use std::path::{Path, PathBuf};

use crate::app_config::StorageFormat;
use enum_dispatch::enum_dispatch;
use eyre::Result;
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, info, instrument};

mod database;
mod json;

use database::StorageDatabase;
use json::StorageJson;

#[derive(Clone)]
pub struct ResourcePath {
    parts: Vec<String>,
}

impl std::fmt::Debug for ResourcePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("ResourcePath({})", self.parts.join("/")))
    }
}

impl ResourcePath {
    pub fn new<T, I>(parts: T) -> Self
    where
        T: IntoIterator<Item = I>,
        I: Into<String>,
    {
        let parts = parts.into_iter().map(Into::into).collect::<Vec<_>>();
        ResourcePath { parts }
    }

    pub fn join<I>(&self, part: I) -> Self
    where
        I: Into<String>,
    {
        let mut parts = self.parts.clone();
        parts.push(part.into());
        ResourcePath { parts }
    }

    pub fn to_json_path(&self, root: impl AsRef<Path>) -> PathBuf {
        let mut path = root.as_ref().to_path_buf();
        for part in &self.parts {
            path = path.join(part);
        }
        path
    }

    pub fn to_db_path(&self, prefix: &str) -> String {
        format!("{}__{}", prefix, self.parts.join("__"))
    }
}

pub trait IntoResourcePath {
    fn into_resource(self) -> Result<ResourcePath>;
}

impl<R, S> IntoResourcePath for R
where
    R: IntoIterator<Item = S>,
    S: Into<String>,
{
    fn into_resource(self) -> Result<ResourcePath> {
        let parts = self.into_iter().map(Into::into).collect::<Vec<_>>();
        Ok(ResourcePath { parts })
    }
}

impl IntoResourcePath for ResourcePath {
    fn into_resource(self) -> Result<ResourcePath> {
        Ok(self)
    }
}

impl IntoResourcePath for &ResourcePath {
    fn into_resource(self) -> Result<ResourcePath> {
        Ok(self.clone())
    }
}

#[enum_dispatch(Storage)]
#[derive(Debug)]
pub enum StorageImpl {
    StorageJson,
    StorageDatabase,
}

#[enum_dispatch]
pub trait Storage {
    /// store a record
    async fn store_record<D>(
        &self,
        path: impl IntoResourcePath,
        id: impl AsRef<str>,
        data: D,
    ) -> Result<()>
    where
        D: Serialize;
    /// store multiple records using an iterator of (id, record) pairs
    async fn store_records<K, D>(
        &self,
        path: impl IntoResourcePath,
        data: impl IntoIterator<Item = (K, D)>,
    ) -> Result<()>
    where
        D: Serialize,
        K: AsRef<str>;
    /// fetch a record
    async fn fetch_record<D>(
        &self,
        path: impl IntoResourcePath,
        id: impl AsRef<str>,
    ) -> Result<Option<D>>
    where
        D: DeserializeOwned;
    /// fetch multiple records taking an iterator of id's
    async fn fetch_records<K, D>(
        &self,
        path: impl IntoResourcePath,
        ids: impl IntoIterator<Item = K>,
    ) -> Result<Vec<(String, D)>>
    where
        K: AsRef<str>,
        D: DeserializeOwned;
}

impl StorageFormat {
    #[instrument]
    pub async fn build(&self) -> Result<StorageImpl> {
        match self {
            StorageFormat::Json { path } => {
                let path = std::path::Path::new(path);
                if !path.exists() {
                    info!(
                        "Raw Metadata directory at {} does not exist, creating it",
                        path.display()
                    );
                    tokio::fs::create_dir_all(path).await?;
                }
                Ok(StorageJson {
                    path: path.to_path_buf(),
                }
                .into())
            }
            StorageFormat::Database {
                connection_url,
                prefix,
            } => {
                use sqlx::migrate::MigrateDatabase;
                if !sqlx::Postgres::database_exists(connection_url)
                    .await
                    .unwrap_or(false)
                {
                    debug!("Database Storage: Creating database? {}", connection_url);
                    sqlx::Postgres::create_database(connection_url).await?;
                }
                let client = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(5)
                    .connect(connection_url)
                    .await?;
                let db = StorageDatabase {
                    connection: connection_url.to_owned(),
                    prefix: prefix.to_owned(),
                    client,
                    known_tables: std::collections::HashSet::new(),
                };

                Ok(db.into())
            }
        }
    }
}

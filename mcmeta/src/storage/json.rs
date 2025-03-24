use eyre::{Context, Result};
use std::path::PathBuf;
use tracing::{info, instrument};

use crate::errors::MetadataError;

use super::{ResourcePath, Storage};
use crate::utils::{deserialize_json_value_with_error_path, parse_json_with_error_context};

#[derive(Debug)]
pub struct StorageJson {
    pub path: PathBuf,
}

impl StorageJson {
    #[instrument(skip(self))]
    async fn load_table_raw(&self, path: &ResourcePath, id: &str) -> Result<serde_json::Value> {
        let path = path.to_json_path(&self.path).join(format!("{}.json", id));
        if !path.exists() {
            return Ok(serde_json::Value::default());
        }
        let table_json = tokio::fs::read_to_string(path).await?;
        if table_json.is_empty() {
            return Ok(serde_json::Value::default());
        }
        let table = parse_json_with_error_context(&table_json)?;
        Ok(table)
    }

    #[instrument(skip(self))]
    #[allow(dead_code)]
    async fn load_table(
        &self,
        path: &ResourcePath,
        id: &str,
    ) -> Result<serde_json::value::Map<String, serde_json::Value>> {
        Ok(self
            .load_table_raw(path, id)
            .await?
            .as_object()
            .cloned()
            .unwrap_or_default())
    }

    #[instrument(skip(self))]
    async fn store_table(
        &self,
        path: &ResourcePath,
        id: &str,
        data: &serde_json::Value,
    ) -> Result<()> {
        let json_data = serde_json::to_string_pretty(&data)?;
        let path = path.to_json_path(&self.path).join(format!("{}.json", id));
        if !path.parent().is_some_and(std::path::Path::exists) {
            let parent = path.parent().unwrap();
            info!(
                "directory at {} does not exist, creating it",
                parent.display()
            );
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| "Failed to create directory")?;
        }
        tokio::fs::write(path, &json_data).await?;
        Ok(())
    }
}

impl Storage for StorageJson {
    async fn store_record<D>(
        &self,
        path: impl super::IntoResourcePath,
        id: impl AsRef<str>,
        data: D,
    ) -> color_eyre::Result<()>
    where
        D: serde::Serialize,
    {
        let path = path.into_resource()?;
        let id = id.as_ref();
        let data_value = serde_json::to_value(&data)?;
        self.store_table(&path, id, &data_value).await
    }

    async fn store_records<K, D>(
        &self,
        path: impl super::IntoResourcePath,
        data: impl IntoIterator<Item = (K, D)>,
    ) -> Result<()>
    where
        D: serde::Serialize,
        K: AsRef<str>,
    {
        let path = path.into_resource()?;
        for (id, record) in data {
            let id = id.as_ref();
            let data_value = serde_json::to_value(&record)?;
            self.store_table(&path, id, &data_value).await?;
        }
        Ok(())
    }

    async fn fetch_record<D>(
        &self,
        path: impl super::IntoResourcePath,
        id: impl AsRef<str>,
    ) -> Result<Option<D>>
    where
        D: serde::de::DeserializeOwned,
    {
        let path = path.into_resource()?;
        let id = id.as_ref();
        let table = self.load_table_raw(&path, id).await?;
        deserialize_json_value_with_error_path(&table)
    }

    async fn fetch_records<K, D>(
        &self,
        path: impl super::IntoResourcePath,
        ids: impl IntoIterator<Item = K>,
    ) -> Result<Vec<(String, D)>>
    where
        K: AsRef<str>,
        D: serde::de::DeserializeOwned,
    {
        let path = path.into_resource()?;
        let ids = ids
            .into_iter()
            .map(|id| id.as_ref().to_string())
            .collect::<Vec<_>>();
        use futures::StreamExt;
        let results = futures::stream::iter(ids)
            .map(|id| async {
                self.load_table_raw(&path, &id)
                    .await
                    .and_then(|r| deserialize_json_value_with_error_path::<D>(&r))
                    .map(|r| (id, r))
            })
            .buffer_unordered(5)
            .collect::<Vec<_>>()
            .await;
        let (records, errors) = crate::utils::process_results(results);
        if !errors.is_empty() {
            Err(
                MetadataError::BulkProcessingError(String::from("Collecting Json Records"), errors)
                    .into(),
            )
        } else {
            Ok(records)
        }
    }
}

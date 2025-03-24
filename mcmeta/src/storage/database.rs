use eyre::Result;
use sqlx::{Execute, Executor};
use tracing::{debug, info, instrument};

use super::{ResourcePath, Storage};
use crate::utils::deserialize_json_with_error_path;

#[derive(Debug)]
pub struct StorageDatabase {
    #[allow(dead_code)]
    pub connection: String,
    pub prefix: String,
    pub client: sqlx::Pool<sqlx::Postgres>,
    pub known_tables: std::collections::HashSet<String>,
}

#[derive(sqlx::FromRow)]
pub struct DatabaseRecord {
    #[allow(dead_code)]
    pub id: String,
    pub record: String,
}

impl StorageDatabase {
    #[instrument(skip(self))]
    pub async fn ensure_table(&self, table_name: &str) -> Result<()> {
        if !self.known_tables.contains(table_name) {
            let mut query = sqlx::QueryBuilder::new("CREATE TABLE IF NOT EXISTS ");
            query.push(table_name);
            query.push(" (id VARCHAR(511) PRIMARY KEY, record JSON);");
            let query = query.build();
            debug!("Database Storage: Running query `{}`", query.sql());
            let res = self.client.execute(query).await?;
            info!("Database Storage: Created Table? {:?}", res);
        }
        Ok(())
    }

    #[instrument(skip(self, records))]
    pub async fn insert_or_update_records(
        &self,
        path: &ResourcePath,
        records: impl IntoIterator<Item = (impl AsRef<str>, serde_json::Value)>,
    ) -> Result<()> {
        let table_name = path.to_db_path(&self.prefix);
        self.ensure_table(&table_name).await?;
        let mut query_builder =
            sqlx::QueryBuilder::new(format!("INSERT INTO {} (id, record) ", &table_name));
        let records = records
            .into_iter()
            .map(|(id, record)| Ok((id.as_ref().to_string(), serde_json::to_string(&record)?)))
            .collect::<Result<Vec<_>>>()?;
        query_builder.push_values(records, |mut b, (id, record)| {
            b.push_bind(id).push_bind(record);
        });
        query_builder.push("ON CONFLICT (id) DO UPDATE SET record = excluded.record");
        let query = query_builder.build();
        debug!("Database Storage: Running query `{}`", query.sql());
        let _ = query.execute(&self.client).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn fetch_record(&self, path: &ResourcePath, id: &str) -> Result<DatabaseRecord> {
        let table_name = path.to_db_path(&self.prefix);
        let mut query_builder = sqlx::QueryBuilder::new(format!(
            "SELECT (id, record) FROM {} WHERE id = ",
            &table_name
        ));
        query_builder.push_bind(id);
        let query = query_builder.build_query_as::<DatabaseRecord>();
        debug!("Database Storage: Running query `{}`", query.sql());
        let record: DatabaseRecord = query.fetch_one(&self.client).await?;
        Ok(record)
    }

    #[instrument(skip(self, ids))]
    pub async fn fetch_records<K>(
        &self,
        path: &ResourcePath,
        ids: impl IntoIterator<Item = K>,
    ) -> Result<Vec<DatabaseRecord>>
    where
        K: AsRef<str>,
    {
        let table_name = path.to_db_path(&self.prefix);
        let mut query_builder = sqlx::QueryBuilder::new(format!(
            "SELECT (id, record) FROM {} WHERE id in (",
            &table_name
        ));
        let mut separated = query_builder.separated(", ");
        for id in ids {
            separated.push_bind(id.as_ref().to_string());
        }
        separated.push_unseparated(") ");

        let query = query_builder.build_query_as::<DatabaseRecord>();
        debug!("Database Storage: Running query `{}`", query.sql());
        let records: Vec<DatabaseRecord> = query.fetch_all(&self.client).await?;
        Ok(records)
    }
}

impl Storage for StorageDatabase {
    async fn store_record<D>(
        &self,
        path: impl super::IntoResourcePath,
        id: impl AsRef<str>,
        data: D,
    ) -> Result<()>
    where
        D: serde::Serialize,
    {
        let path = path.into_resource()?;
        let id = id.as_ref();
        let data_value = serde_json::to_value(&data)?;
        self.insert_or_update_records(&path, vec![(id, data_value)])
            .await
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
        self.insert_or_update_records(
            &path,
            data.into_iter()
                .map(|(id, record)| Ok((id, serde_json::to_value(&record)?)))
                .collect::<Result<Vec<_>>>()?,
        )
        .await
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
        let record = self.fetch_record(&path, id).await?;
        deserialize_json_with_error_path(&record.record)
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
        let records = self.fetch_records(&path, ids).await?;
        records
            .into_iter()
            .map(|r| deserialize_json_with_error_path(&r.record))
            .collect::<Result<Vec<_>>>()
    }
}

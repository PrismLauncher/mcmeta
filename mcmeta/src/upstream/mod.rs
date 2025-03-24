pub mod forge;
pub mod mojang;

use eyre::Result;
use forge::ForgeUpdater;
use mojang::MojangUpdater;
use std::{path::PathBuf, sync::Arc};
use tracing::error;

use crate::{app_config::UpstreamConfig, storage::StorageImpl, UpstreamSources};

pub fn progress_bar() -> indicatif::ProgressStyle {
    indicatif::ProgressStyle::with_template(
        "{spinner:.green} {msg} [{wide_bar:.cyan/blue}] {pos}/{len} (eta {eta})",
    )
    .unwrap()
    .with_key(
        "eta",
        |state: &indicatif::ProgressState, w: &mut dyn std::fmt::Write| {
            write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
        },
    )
}

pub async fn download_binary_file(
    client: &reqwest::Client,
    path: &PathBuf,
    url: &str,
) -> Result<()> {
    if let Some(parent_dir) = path.parent() {
        if !parent_dir.exists() {
            std::fs::create_dir_all(parent_dir)?;
        }
    }

    let file_response = client.get(url).send().await?.error_for_status()?;

    let mut file = std::fs::File::create(path)?;
    let mut content = std::io::Cursor::new(file_response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;

    Ok(())
}

pub struct UpstreamMetadataUpdater {
    storage: Arc<StorageImpl>,
    config: UpstreamConfig,
}

impl UpstreamMetadataUpdater {
    pub fn new(config: UpstreamConfig, storage: Arc<StorageImpl>) -> Self {
        UpstreamMetadataUpdater { storage, config }
    }
    pub async fn update(&self, sources: &[UpstreamSources]) -> Result<()> {
        for source in sources {
            match source {
                UpstreamSources::All => unreachable!("All type marker should be filtered out"),
                UpstreamSources::Mojang => {
                    let res = MojangUpdater::new(self.storage.clone(), self.config.clone())
                        .update()
                        .await;
                    if let Err(err) = res {
                        error!("Error updating Mojang metadata: {:?}", err);
                    }
                }
                UpstreamSources::Forge => {
                    let res = ForgeUpdater::new(self.storage.clone(), self.config.clone())
                        .update()
                        .await;
                    if let Err(err) = res {
                        error!("Error updating Forge metadata: {:?}", err);
                    }
                }
            }
        }
        Ok(())
    }
}

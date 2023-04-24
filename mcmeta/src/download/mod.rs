pub mod errors;
pub mod forge;
pub mod mojang;

use anyhow::Result;
use std::path::PathBuf;

pub async fn download_binary_file(path: &PathBuf, url: &str) -> Result<()> {
    let client = reqwest::Client::new();

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

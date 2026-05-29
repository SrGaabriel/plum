use async_compression::tokio::bufread::GzipDecoder;
use futures_util::TryStreamExt;
use fxhash::FxHashMap;
use reqwest::{Client, header::ACCEPT};
use serde::{Deserialize, Serialize};
use tokio::io::BufReader;
use tokio_tar::Archive;
use tokio_util::io::StreamReader;

const HACKAGE_URL: &str = "https://hackage.haskell.org";

fn mk_url(endpoint: &str) -> String {
    format!("{HACKAGE_URL}/{endpoint}")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum VersionStatus {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "deprecated")]
    Deprecated,
}

type PackageVersions = FxHashMap<String, VersionStatus>;

pub async fn get_versions(
    client: &Client,
    package_name: &str,
) -> Result<PackageVersions, reqwest::Error> {
    let url = mk_url(&format!("package/{package_name}"));
    let response = client
        .get(&url)
        .header(ACCEPT, "application/json")
        .send()
        .await?;
    let body = response.json().await?;
    Ok(body)
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("Failed to download tarball: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Failed to read tarball stream: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn download_tarball(
    client: &Client,
    package_name: &str,
    version: &str,
    path: &str,
) -> Result<(), DownloadError> {
    let url = mk_url(&format!(
        "package/{package_name}-{version}/{package_name}-{version}.tar.gz"
    ));
    let response = client.get(&url).send().await?.error_for_status()?;

    let stream = response.bytes_stream().map_err(std::io::Error::other);
    let reader = StreamReader::new(stream);
    let decoder = GzipDecoder::new(reader);
    let buffered = BufReader::new(decoder);
    let mut archive = Archive::new(buffered);

    archive.unpack(path).await.map_err(DownloadError::Io)?;
    Ok(())
}

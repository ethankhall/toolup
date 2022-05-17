use crate::model::{AuthStrategy, PackageRepository, RemotePackage, LocalPackageRepository, S3PackageRepository};
use crate::util::{extract_env_from_script, GlobalFolders};
use async_trait::async_trait;
use reqwest::header::ETAG;
use rusoto_core::region::Region;
use rusoto_core::signature::SignedRequest;
use rusoto_credential::ChainProvider;
use rusoto_credential::ProvideAwsCredentials;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info};
use url::Url;

#[derive(Error, Debug)]
pub enum RemoteError {
    #[error(transparent)]
    State(#[from] crate::state::StateError),
    #[error(transparent)]
    Cred(#[from] rusoto_credential::CredentialsError),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Uknown(#[from] anyhow::Error),
}

#[derive(Debug)]
pub struct DownloadedArtifact {
    pub path: PathBuf,
    pub etag: Option<String>,
}

#[async_trait]
pub trait RemoteDownload {
    async fn download(
        &self,
        remote: &RemotePackage,
        global_folder: &GlobalFolders,
    ) -> Result<DownloadedArtifact, RemoteError>;

    async fn needs_update(
        &self,
        remote: &RemotePackage,
        etag: Option<String>,
    ) -> Result<bool, RemoteError>;
}

#[async_trait]
impl RemoteDownload for S3PackageRepository {
    async fn needs_update(
        &self,
        _remote: &RemotePackage,
        etag: Option<String>,
    ) -> Result<bool, RemoteError> {
        let etag_string = match etag {
            None => {
                return Ok(true);
            }
            Some(value) => value,
        };

        let signed_url = self.make_presigned_url("HEAD").await?;
        let response = reqwest::Client::builder()
            .build()?
            .head(signed_url)
            .header("If-None-Match", etag_string)
            .send()
            .await;

        let status_code = match response {
            Err(err) => err.status(),
            Ok(resp) => Some(resp.status()),
        };

        match status_code {
            Some(reqwest::StatusCode::NOT_MODIFIED) => return Ok(false),
            _ => return Ok(true),
        }
    }

    async fn download(
        &self,
        remote: &RemotePackage,
        global_folder: &GlobalFolders,
    ) -> Result<DownloadedArtifact, RemoteError> {
        let signed_url = self.make_presigned_url("GET").await?;
        let response = reqwest::get(signed_url).await?;
        let headers = response.headers();
        let etag = headers
            .get(ETAG)
            .map(|etag| etag.to_str().ok())
            .flatten()
            .map(|etag| etag.to_owned());
        let bytes = response.bytes().await?;

        let now = chrono::Utc::now();
        let mut path = global_folder.get_remote_download_dir();

        if !path.exists() {
            fs::create_dir_all(path.clone())?;
        }

        path.push(format!("{}.download.{}", remote.name, now.timestamp()));
        let mut file = File::create(&path)?;
        file.write_all(&bytes)?;

        info!("Artifact saved to {}", path.display().to_string());
        Ok(DownloadedArtifact {
            path: path.clone(),
            etag,
        })
    }
}

impl S3PackageRepository {
    async fn make_presigned_url(&self, method: &str) -> Result<String, RemoteError> {
        let extra_env = match &self.auth_strategy {
            AuthStrategy::Script(auth_script) => extract_env_from_script(auth_script)?,
            AuthStrategy::None => BTreeMap::default(),
        };

        if !extra_env.is_empty() {
            debug!("Expecting inject {:?}", extra_env);
        }

        for (name, value) in extra_env {
            std::env::set_var(name, value);
        }

        info!("Downloading {}", self.url);

        let url = Url::parse(&self.url)?;
        let domain = url.domain().expect("URL to have a domain name").to_string();
        let region = Region::Custom {
            name: "custom-domain".to_owned(),
            endpoint: domain,
        };

        let mut request = SignedRequest::new(method, "s3", &region, url.path());
        for (name, value) in url.query_pairs() {
            request.add_param(name, value);
        }

        let creds = ChainProvider::default();
        debug!("{:?}", creds);
        let minute = std::time::Duration::from_secs(60);

        let signed_url =
            request.generate_presigned_url(&creds.credentials().await?, &minute, false);
        debug!("Signed URL: {}", signed_url);

        Ok(signed_url)
    }
}

#[async_trait]
impl RemoteDownload for LocalPackageRepository {
    async fn needs_update(
        &self,
        _remote: &RemotePackage,
        _etag: Option<String>,
    ) -> Result<bool, RemoteError> {
        Ok(true)
    }

    async fn download(
        &self,
        _remote: &RemotePackage,
        _global_folder: &GlobalFolders,
    ) -> Result<DownloadedArtifact, RemoteError> {
        let path = PathBuf::from(&self.path);

        info!("Artifact from to {}", self.path);
        Ok(DownloadedArtifact {
            path: path.clone(),
            etag: None,
        })
    }
}

pub async fn update_remote(
    remote: RemotePackage,
    global_folder: &GlobalFolders,
) -> Result<DownloadedArtifact, RemoteError> {
    match &remote.repository {
        PackageRepository::S3(s3) => s3.download(&remote, global_folder).await,
        PackageRepository::Local(local) => local.download(&remote, global_folder).await,
    }
}

pub async fn package_needs_update(
    remote: &RemotePackage,
    etag: Option<String>,
) -> Result<bool, RemoteError> {
    match &remote.repository {
        PackageRepository::S3(s3) => s3.needs_update(&remote, etag).await,
        PackageRepository::Local(local) => local.needs_update(&remote, etag).await,
    }
}

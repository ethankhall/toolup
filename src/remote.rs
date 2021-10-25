use crate::model::{AuthStrategy, PackageRepository, RemotePackage, S3PackageRepository};
use crate::util::{extract_env_from_script, GlobalFolders};
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

pub async fn update_remote(
    remote: RemotePackage,
    global_folder: &GlobalFolders,
) -> Result<PathBuf, RemoteError> {
    match &remote.repository {
        PackageRepository::S3(s3) => process_s3_repo(&remote, s3, global_folder).await,
    }
}

async fn process_s3_repo(
    remote: &RemotePackage,
    repo: &S3PackageRepository,
    global_folder: &GlobalFolders,
) -> Result<PathBuf, RemoteError> {
    let extra_env = match &repo.auth_strategy {
        AuthStrategy::Script(auth_script) => extract_env_from_script(auth_script)?,
        AuthStrategy::None => BTreeMap::default(),
    };

    if !extra_env.is_empty() {
        debug!("Expecting inject {:?}", extra_env);
    }

    for (name, value) in extra_env {
        std::env::set_var(name, value);
    }

    info!("Downloading {}", repo.url);

    let url = Url::parse(&repo.url)?;
    let domain = url.domain().expect("URL to have a domain name").to_string();
    let region = Region::Custom {
        name: "custom-domain".to_owned(),
        endpoint: domain,
    };

    let mut request = SignedRequest::new("GET", "s3", &region, url.path());
    for (name, value) in url.query_pairs() {
        request.add_param(name, value);
    }

    let creds = ChainProvider::default();
    debug!("{:?}", creds);
    let minute = std::time::Duration::from_secs(60);

    let signed_url = request.generate_presigned_url(&creds.credentials().await?, &minute, false);
    debug!("Signed URL: {}", signed_url);
    let bytes = reqwest::get(signed_url).await?.bytes().await?;

    let now = chrono::Utc::now();
    let mut path = global_folder.get_remote_download_dir();

    if !path.exists() {
        fs::create_dir_all(path.clone())?;
    }

    path.push(format!("{}.download.{}", remote.name, now.timestamp()));
    let mut file = File::create(&path)?;
    file.write_all(&bytes)?;

    info!("Artifact saved to {}", path.display().to_string());
    Ok(path.clone())
}

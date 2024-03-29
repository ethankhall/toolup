use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::model::RemotePackage;
use crate::package::{install_package, PackageError};
use crate::remote::{package_needs_update, update_remote};
use crate::state::{get_current_state, update_links, PackageDescription};
use crate::util::GlobalFolders;
use async_trait::async_trait;
use std::fs;
use thiserror::Error;
use tracing::{debug, info, instrument};

#[derive(Error, Debug)]
pub enum UpdateRemoteError {
    #[error(transparent)]
    Package(#[from] PackageError),
    #[error(transparent)]
    Remote(#[from] crate::remote::RemoteError),
    #[error(transparent)]
    State(#[from] crate::state::StateError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Unkown(#[from] anyhow::Error),
    #[error("Application has not been configured")]
    NoGlobalStateFile,
}

#[async_trait]
impl SubCommandExec<UpdateRemoteError> for UpdateRemoteSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), UpdateRemoteError> {
        debug!("Executing update");
        let remote_folder = global_folder.get_remote_config_dir();
        let global_state = global_folder.global_state_file();
        let container = get_current_state(&global_state).await?;

        if !remote_folder.exists() {
            std::fs::create_dir_all(&remote_folder)?;
        }

        for entry in fs::read_dir(remote_folder)? {
            let entry = entry?;
            debug!("Processing remote file {:?}", entry.path());
            let contents = fs::read_to_string(entry.path())?;
            let remote_package: RemotePackage = serde_json::from_str(&contents)?;
            let package_name = remote_package.name.clone();
            let installed_package = container.describe_package(&package_name);
            if self.only == None || self.only == Some(package_name) {
                update_package(remote_package, installed_package, global_folder).await?;
            }
        }

        let container = get_current_state(&global_state).await?;
        update_links(&container, global_folder).await?;

        Ok(())
    }
}

#[instrument(skip_all, fields(packge=%remote_package.name))]
async fn update_package(
    remote_package: RemotePackage,
    installed_package: Option<PackageDescription>,
    global_folder: &GlobalFolders,
) -> Result<(), UpdateRemoteError> {
    info!(target: "user", "Updating {}", remote_package.name);
    debug!(remote_package=?remote_package, installed_package=?installed_package);
    let etag = match installed_package {
        None => None,
        Some(pacakge) => pacakge.etag,
    };

    if package_needs_update(&remote_package, etag).await? {
        info!(target: "user", "Downloading {} from remote.", &remote_package.name);
        let artifact = update_remote(remote_package, global_folder).await?;
        install_package(&artifact, true, global_folder).await?;
        debug!("Removing file {:?}", artifact);
        fs::remove_file(&artifact.path)?;
    } else {
        info!(target: "user", "Package was already up-to-date, skipping update.");
    }

    Ok(())
}

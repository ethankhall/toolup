use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::model::{RemotePackage};
use crate::remote::update_remote;
use crate::util::{GlobalFolders};
use async_trait::async_trait;
use std::fs;
use thiserror::Error;
use tracing::{debug, info};
use crate::package::{PackageError, install_package};

#[derive(Error, Debug)]
pub enum UpdateRemoteError {
    #[error(transparent)]
    PackageError(#[from] PackageError),
    #[error(transparent)]
    RemoteError(#[from] crate::remote::RemoteError),
    #[error(transparent)]
    StateError(#[from] crate::state::StateError),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[async_trait]
impl SubCommandExec<UpdateRemoteError> for UpdateRemoteSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), UpdateRemoteError> {
        let remote_folder = global_folder.get_remote_config_dir();
        for entry in fs::read_dir(remote_folder)? {
            let entry = entry?;
            debug!("Processing remote file {:?}", entry.path());
            let contents = fs::read_to_string(entry.path())?;
            let remote_package: RemotePackage = serde_json::from_str(&contents)?;
            if self.only == None || self.only == Some(remote_package.name.clone()) {
                info!(target: "user", "Updating {}", remote_package.name);
                let artifact = update_remote(remote_package, global_folder).await?;
                install_package(&artifact, true, global_folder).await?;
                debug!("Removing file {:?}", artifact);
                fs::remove_file(&artifact)?;
            }
        }

        Ok(())
    }
}

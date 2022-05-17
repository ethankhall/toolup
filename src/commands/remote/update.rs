use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::model::RemotePackage;
use crate::package::{install_package, PackageError};
use crate::remote::{package_needs_update, update_remote};
use crate::state::{get_current_state, update_links};
use crate::util::GlobalFolders;
use async_trait::async_trait;
use std::fs;
use thiserror::Error;
use tracing::{debug, info};

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
    Uknown(#[from] anyhow::Error),
}

#[async_trait]
impl SubCommandExec<UpdateRemoteError> for UpdateRemoteSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), UpdateRemoteError> {
        let remote_folder = global_folder.get_remote_config_dir();
        let global_state = global_folder.global_state_file();
        let container = get_current_state(&global_state).await?;

        for entry in fs::read_dir(remote_folder)? {
            let entry = entry?;
            debug!("Processing remote file {:?}", entry.path());
            let contents = fs::read_to_string(entry.path())?;
            let remote_package: RemotePackage = serde_json::from_str(&contents)?;
            if self.only == None || self.only == Some(remote_package.name.clone()) {
                info!(target: "user", "Updating {}", remote_package.name);
                let etag = match container
                    .current_state
                    .current_packages
                    .get(&remote_package.name)
                {
                    None => None,
                    Some(pacakge) => pacakge.etag.clone(),
                };

                if package_needs_update(&remote_package, etag).await? {
                    let artifact = update_remote(remote_package, global_folder).await?;
                    install_package(&artifact, true, global_folder).await?;
                    debug!("Removing file {:?}", artifact);
                    fs::remove_file(&artifact.path)?;
                } else {
                    debug!("Package was already up-to-date, skipping update.")
                }
            }
        }

        let container = get_current_state(&global_state).await?;
        update_links(&container, global_folder).await?;

        Ok(())
    }
}

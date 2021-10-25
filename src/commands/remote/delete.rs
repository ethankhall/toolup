use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::state::{get_current_state, write_state};
use crate::util::GlobalFolders;
use async_trait::async_trait;
use std::fs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeleteRemoteError {
    #[error("Remote {name} was not found on the system.")]
    RemoteNotFound { name: String },
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
impl SubCommandExec<DeleteRemoteError> for DeleteRemoteSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), DeleteRemoteError> {
        let config_file = global_folder.make_remote_tool_config(&self.name);
        if config_file.exists() {
            fs::remove_file(config_file)?;
        } else {
            return Err(DeleteRemoteError::RemoteNotFound { name: self.name });
        }

        if self.cascade {
            let global_state = global_folder.global_state_file();
            let mut container = get_current_state(&global_state).await?;
            let mut current_state = container.current_state;
            let mut packages_to_remove = Vec::new();
            for package in &current_state.installed_packages {
                if package.remote_name == Some(self.name.clone()) {
                    packages_to_remove.push(package.clone());
                }
            }

            current_state.remove_packages(packages_to_remove);
            container.current_state = current_state;
            write_state(&global_state, container).await?;
        }
        Ok(())
    }
}

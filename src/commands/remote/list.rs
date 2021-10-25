use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::model::RemotePackage;
use crate::state::get_current_state;
use crate::util::GlobalFolders;
use async_trait::async_trait;
use std::fs;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum ListRemoteError {
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
impl SubCommandExec<ListRemoteError> for ListRemoteSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), ListRemoteError> {
        let global_state = global_folder.global_state_file();
        let container = get_current_state(&global_state).await?;
        let current_state = container.current_state;

        let remote_folder = global_folder.get_remote_config_dir();
        for entry in fs::read_dir(remote_folder)? {
            let contents = fs::read_to_string(entry?.path())?;
            let remote_package: RemotePackage = serde_json::from_str(&contents)?;
            info!(target: "user", "{} is sourced from {}", remote_package.name, remote_package.repository);
            for package in &current_state.installed_packages {
                if package.remote_name == Some(remote_package.name.clone()) {
                    let description = current_state.describe_package(package);
                    let description: Vec<String> = description
                        .binaries
                        .iter()
                        .map(|(key, value)| {
                            if *value {
                                format!("{} (current)", key)
                            } else {
                                key.to_string()
                            }
                        })
                        .collect();
                    info!(target: "user", "  {}@{} provides {}", package.name, package.version, description.join(", "));
                }
            }
        }
        Ok(())
    }
}

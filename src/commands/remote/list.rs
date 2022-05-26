use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::model::RemotePackage;
use crate::state::get_current_state;
use crate::util::GlobalFolders;
use async_trait::async_trait;
use std::fs::{self, DirEntry};
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

        let remote_folder = global_folder.get_remote_config_dir();

        if !remote_folder.exists() {
            info!(target: "user", "No remote configurations exist.");
            return Ok(());
        }

        let mut remote_configs: Vec<DirEntry> = Default::default();

        for entry in fs::read_dir(&remote_folder)?.flatten() {
            remote_configs.push(entry);
        }

        if remote_configs.is_empty() {
            info!(target: "user", "No remote configurations exist in {}", remote_folder.display());
        }

        for entry in remote_configs {
            let contents = fs::read_to_string(entry.path())?;
            let remote_package: RemotePackage = serde_json::from_str(&contents)?;
            info!(target: "user", "{} is sourced from {}", remote_package.name, remote_package.repository);
            for package in &container.list_installed_packages() {
                if package.remote_name == Some(remote_package.name.clone()) {
                    let description: Vec<String> = package
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

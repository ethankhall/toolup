use async_trait::async_trait;
use std::path::*;
use thiserror::Error;

use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::package::{install_package, PackageError};
use crate::remote::DownloadedArtifact;
use crate::state::{get_current_state, update_links};
use crate::util::GlobalFolders;

#[derive(Error, Debug)]
pub enum InstallPackageError {
    #[error(transparent)]
    State(#[from] crate::state::StateError),
    #[error(transparent)]
    Package(#[from] PackageError),
    #[error(transparent)]
    Uknown(#[from] anyhow::Error),
}

#[async_trait]
impl SubCommandExec<InstallPackageError> for InstallToolSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), InstallPackageError> {
        let archive_path = DownloadedArtifact {
            path: PathBuf::from(&self.archive_path),
            etag: None,
        };
        install_package(&archive_path, self.overwrite, global_folder).await?;

        let global_state = global_folder.global_state_file();
        let container = get_current_state(&global_state).await?;
        update_links(&container, global_folder).await?;

        Ok(())
    }
}

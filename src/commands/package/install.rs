use async_trait::async_trait;
use std::path::*;
use thiserror::Error;

use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::util::{GlobalFolders};
use crate::package::{PackageError, install_package};
use crate::state::{update_links, get_current_state};

#[derive(Error, Debug)]
pub enum InstallPackageError {
    #[error(transparent)]
    StateError(#[from] crate::state::StateError),
    #[error(transparent)]
    PackageError(#[from] PackageError),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[async_trait]
impl SubCommandExec<InstallPackageError> for InstallToolSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), InstallPackageError> {
        let archive_path = Path::new(&self.archive_path);
        install_package(archive_path, self.overwrite, global_folder).await?;

        let global_state = global_folder.global_state_file();
        let container = get_current_state(&global_state).await?;
        update_links(&container, global_folder).await?;

        Ok(())
    }
}

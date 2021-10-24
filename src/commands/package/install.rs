use async_trait::async_trait;
use std::path::*;
use thiserror::Error;

use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::util::{GlobalFolders};
use crate::package::{PackageError, install_package};

#[derive(Error, Debug)]
pub enum InstallPackageError {
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
        Ok(())
    }
}

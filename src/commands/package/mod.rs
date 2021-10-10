mod create;
mod init;
mod install;

use crate::PackageSubCommand;
use crate::commands::SubCommandExec;
use thiserror::Error;

pub mod prelude {
    pub use super::create::{ArchivePackageError};
    pub use super::init::{InitPackageError};
    pub use super::install::{InstallPackageError};
    pub use super::handle_package;
    pub use super::PackageError;
}

use prelude::*;

#[derive(Error, Debug)]
pub enum PackageError {
    #[error(transparent)]
    ArchiveError(#[from] ArchivePackageError),
    #[error(transparent)]
    InitError(#[from] InitPackageError),
    #[error(transparent)]
    InstallError(#[from] InstallPackageError),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

pub async fn handle_package(package_sub_args: PackageSubCommand) -> Result<(), PackageError> {
    match package_sub_args {
        PackageSubCommand::Init(args) => args.execute().await?,
        PackageSubCommand::Archive(args) => args.execute().await?,
        PackageSubCommand::Install(args) => args.execute().await?,
    };

    Ok(())
}
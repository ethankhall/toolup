mod create;
mod init;
mod install;

use crate::cli::PackageSubCommand;
use crate::commands::SubCommandExec;
use crate::util::GlobalFolders;
pub use create::ArchivePackageError;
pub use init::InitPackageError;
pub use install::InstallPackageError;
use thiserror::Error;

pub mod prelude {
    pub use super::create::ArchivePackageError;
    pub use super::handle_package;
    pub use super::init::InitPackageError;
    pub use super::install::InstallPackageError;
    pub use super::PackageError;
}

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

pub async fn handle_package(
    package_sub_args: PackageSubCommand,
    global_folder: &GlobalFolders,
) -> Result<(), PackageError> {
    match package_sub_args {
        PackageSubCommand::Init(args) => args.execute(global_folder).await?,
        PackageSubCommand::Archive(args) => args.execute(global_folder).await?,
        PackageSubCommand::Install(args) => args.execute(global_folder).await?,
    };

    Ok(())
}

use crate::util::GlobalFolders;
use async_trait::async_trait;
use thiserror::Error;

mod config;
mod exec;
mod package;
mod remote;

pub use config::prelude::*;
pub use exec::prelude::*;
pub use package::prelude::*;
pub use remote::prelude::*;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error(transparent)]
    RemoteError(#[from] RemoteError),
    #[error(transparent)]
    PackageError(#[from] PackageError),
    #[error(transparent)]
    ExecError(#[from] ExecError),
    #[error(transparent)]
    ConfigError(#[from] ConfigError),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[async_trait]
pub trait SubCommandExec<E> {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), E>;
}

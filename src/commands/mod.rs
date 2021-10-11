use crate::util::GlobalFolders;
use async_trait::async_trait;
use thiserror::Error;

mod exec;
mod package;

pub use exec::prelude::*;
pub use package::prelude::*;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error(transparent)]
    PackageError(#[from] PackageError),
    #[error(transparent)]
    ExecError(#[from] ExecError),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[async_trait]
pub trait SubCommandExec<E> {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), E>;
}

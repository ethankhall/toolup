use async_trait::async_trait;
use thiserror::Error;

mod package;

pub use package::prelude::*;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error(transparent)]
    PackageError(#[from] PackageError),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[async_trait]
pub trait SubCommandExec<E> {
    async fn execute(self) -> Result<(), E>;
}

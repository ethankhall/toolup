use async_trait::async_trait;
use thiserror::Error;

use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::util::GlobalFolders;

#[derive(Error, Debug)]
pub enum GetLinkPackageError {
    #[error(transparent)]
    Toml(#[from] toml::ser::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Uknown(#[from] anyhow::Error),
}

#[async_trait]
impl SubCommandExec<GetLinkPackageError> for GetPathSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), GetLinkPackageError> {
        println!("{}", global_folder.get_link_dir().display());
        Ok(())
    }
}

mod add;
mod delete;
mod list;
mod update;

use crate::cli::RemoteSubCommand;
use crate::commands::SubCommandExec;
use crate::util::GlobalFolders;
use add::AddRemoteError;
use delete::DeleteRemoteError;
use list::ListRemoteError;
use thiserror::Error;
use update::UpdateRemoteError;

pub mod prelude {
    pub use super::handle_remote;
    pub use super::RemoteError;
}

#[derive(Error, Debug)]
pub enum RemoteError {
    #[error(transparent)]
    AddRemote(#[from] AddRemoteError),
    #[error(transparent)]
    DeleteRemote(#[from] DeleteRemoteError),
    #[error(transparent)]
    ListRemote(#[from] ListRemoteError),
    #[error(transparent)]
    UpdateRemote(#[from] UpdateRemoteError),
    #[error(transparent)]
    Uknown(#[from] anyhow::Error),
}

pub async fn handle_remote(
    manage_sub_args: RemoteSubCommand,
    global_folder: &GlobalFolders,
) -> Result<(), RemoteError> {
    match manage_sub_args {
        RemoteSubCommand::Add(args) => args.execute(global_folder).await?,
        RemoteSubCommand::Delete(args) => args.execute(global_folder).await?,
        RemoteSubCommand::List(args) => args.execute(global_folder).await?,
        RemoteSubCommand::Update(args) => args.execute(global_folder).await?,
    };

    Ok(())
}

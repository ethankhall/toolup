use crate::cli::ExecSubCommand;
use crate::state::*;
use crate::util::exec;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecError {
    #[error(transparent)]
    StateError(#[from] crate::state::StateError),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

pub async fn handle_exec(
    exec_args: ExecSubCommand,
    global_folder: &crate::util::GlobalFolders,
) -> Result<(), ExecError> {
    let global_state = global_folder.global_state_file();
    let container = get_current_state(&global_state).await?;

    let path = match exec_args.version {
        Some(version) => container
            .current_state
            .get_binary_path(&exec_args.command_name, &version)?,
        None => container
            .current_state
            .get_current_binary_path(&exec_args.command_name)?,
    };

    exec(path, exec_args.args);

    unreachable!();
}

pub mod prelude {
    pub use super::{handle_exec, ExecError};
}

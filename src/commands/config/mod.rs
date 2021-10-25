use crate::cli::ConfigSubCommand;
use crate::commands::SubCommandExec;
use thiserror::Error;

mod get_link;

use get_link::GetLinkPackageError;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error(transparent)]
    GetLinkPackageError(#[from] GetLinkPackageError),
}

pub async fn handle_config(
    config_sub_args: ConfigSubCommand,
    global_folder: &crate::util::GlobalFolders,
) -> Result<(), ConfigError> {
    match config_sub_args {
        ConfigSubCommand::GetLinkPath(args) => args.execute(global_folder).await?,
    };

    Ok(())
}

pub mod prelude {
    pub use super::get_link::GetLinkPackageError;
    pub use super::{handle_config, ConfigError};
}

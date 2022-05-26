mod cli;
mod commands;
mod model;
mod package;
mod remote;
mod state;
mod util;

pub mod prelude {
    pub use crate::cli::*;
    pub use crate::commands::{
        handle_config, handle_exec, handle_package, handle_remote, print_version, CommandError,
    };
    pub use crate::state::get_current_state;
    pub use crate::util::{exec, GlobalFolders};
}

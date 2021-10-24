mod cli;
mod commands;
mod model;
mod remote;
mod state;
mod util;
mod package;

pub mod prelude {
    pub use crate::cli::*;
    pub use crate::commands::{handle_exec, handle_package, handle_remote, CommandError};
    pub use crate::state::get_current_state;
    pub use crate::util::{exec, GlobalFolders};
}

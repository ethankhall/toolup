mod cli;
mod commands;
mod model;
mod state;
mod util;

pub mod prelude {
    pub use crate::cli::*;
    pub use crate::commands::{handle_exec, handle_package, CommandError};
    pub use crate::state::get_current_state;
    pub use crate::util::{exec, GlobalFolders};
}

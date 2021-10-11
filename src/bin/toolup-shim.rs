use anyhow::Result as AnyResult;
use std::env;
use std::path::Path;
use toolup::prelude::{exec, get_current_state, GlobalFolders};

#[tokio::main]
async fn main() -> AnyResult<()> {
    dotenv::dotenv().ok();
    human_panic::setup_panic!();

    let global_folder = GlobalFolders::shim_from_env();
    let mut args = Vec::new();
    for arg in env::args() {
        args.push(arg);
    }

    let command = args.remove(0);

    let command = Path::new(&command)
        .file_name()
        .expect("The state file to have a valid filename")
        .to_os_string()
        .into_string()
        .expect("State file to have a valid filename.");

    let global_state = global_folder.global_state_file();

    let container = get_current_state(&global_state).await?;
    let path = container.current_state.get_current_binary_path(&command)?;

    exec(path, args);

    Ok(())
}

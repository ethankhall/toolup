use std::path::Path;
use std::fs;
use std::ffi::OsString;
use std::collections::HashSet;

use crate::common::error::*;
use crate::storage::lock::*;

pub fn update_links(versions: &Vec<ToolVersion>) -> Result<(), CliError> {
    let path_dir = Path::new(crate::PATH_DIR.as_str());
    let mut existing_tools: HashSet<OsString> = HashSet::new();

    if let Ok(dirs) = fs::read_dir(path_dir) {
        for entry in dirs {
            if let Ok(entry) = entry {
                existing_tools.insert(entry.file_name());
            }
        }
    }

    for version in versions {
        let exec_path = Path::new(&version.exec_path);
        let latest_path = path_dir.join(exec_path.file_name().unwrap());

        if latest_path.exists() {
            if let Err(e) = fs::remove_file(&latest_path) {
                eprintln!("Unable to remove {}", e);
            }
        }

        if let Err(e) = link(exec_path, latest_path) {
            eprintln!("Unable to update {:?}, {}", version.exec_path(), e.to_string());
        }
    }

    Ok(())
}

#[cfg(target_family = "unix")]
fn link<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dest: Q) -> std::io::Result<()> {
    std::os::unix::fs::symlink(exec_path, latest_path)
}

#[cfg(target_family = "windows")]
fn link<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dest: Q) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(src, dest)
}
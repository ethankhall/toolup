use std::path::Path;
use std::fs;
use std::ffi::OsString;
use std::collections::HashSet;

use crate::common::error::*;
use crate::storage::lock::*;

pub fn update_links(versions: &Vec<ToolVersion>) -> Result<(), CliError> {
    let path_dir = Path::new(crate::PATH_DIR.as_str());
    let mut existing_tools: HashSet<OsString> = HashSet::new();

    fs::create_dir_all(&path_dir)?;

    if let Ok(dirs) = fs::read_dir(path_dir) {
        for entry in dirs {
            if let Ok(entry) = entry {
                existing_tools.insert(entry.file_name());
            }
        }
    }

    for version in versions {
        let exec_path = version.exec_path();
        let latest_path = path_dir.join(exec_path.file_name().unwrap());

        debug!("Linking {:?} -> {:?}", &exec_path, &latest_path);

        if latest_path.exists() {
            let mut del_file = latest_path.to_path_buf();
            let mut file_name = del_file.file_name().unwrap().to_os_string();
            file_name.push(".del");

            del_file.set_file_name(file_name);
            if let Err(e) = fs::rename(&latest_path, &del_file) {
                eprintln!("Unable to rename link: {}", e);
            } else if let Err(e) = fs::remove_file(&del_file) {
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
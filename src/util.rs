use directories::ProjectDirs;
use lazy_static::lazy_static;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

lazy_static! {
    pub static ref LOG_DIR: String = {
        let project_dirs =
            ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
        project_dirs.data_dir().to_str().unwrap().to_owned()
    };
    pub static ref CONFIG_DIR: String = {
        let project_dirs =
            ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
        project_dirs.config_dir().to_str().unwrap().to_owned()
    };
    pub static ref DOWNLOAD_DIR: String = {
        let project_dirs =
            ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
        let download_dir = Path::join(project_dirs.cache_dir(), "download");
        download_dir.to_str().unwrap().to_owned()
    };
    pub static ref LATEST_INSTALL_DIR: String = {
        let project_dirs =
            ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
        let latest_dir = Path::join(project_dirs.cache_dir(), "latest");
        latest_dir.to_str().unwrap().to_owned()
    };
}

pub fn get_hash_for_contents(input: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();

    format!("{:x}", result)
}

#[cfg(target_family = "unix")]
pub fn set_executable(path: &PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    debug!("Setting {:?} as executable", path);

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o744);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(target_family = "windows")]
pub fn set_executable(_path: &PathBuf) {}

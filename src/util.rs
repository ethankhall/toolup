use crate::cli::{GlobalConfig, LoggingOpts};
use directories::ProjectDirs;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;
use tracing::level_filters::LevelFilter;

pub const GLOBAL_STATE_FILE_NAME: &str = "global-state.json";
pub const TOOL_REMOTE_DIR: &str = "remote.d";
pub const TOOL_DOWNLOAD_DIR: &str = "remote-download";
pub const TOOLUP_GLOBAL_CONFIG_DIR: &str = "TOOLUP_GLOBAL_CONFIG_DIR";
pub const TOOLUP_ROOT_TOOL_DIR: &str = "TOOLUP_ROOT_TOOL_DIR";
pub const TOOL_LINK_FOLDER_NAME: &str = "_bin";

#[derive(Debug)]
pub struct GlobalFolders {
    pub log_dir: String,
    pub config_dir: String,
    pub tool_root_dir: String,
}

impl GlobalFolders {
    pub fn global_state_file(&self) -> PathBuf {
        Path::new(&self.config_dir).join(GLOBAL_STATE_FILE_NAME)
    }

    pub fn make_remote_tool_config(&self, name: &str) -> PathBuf {
        self.get_remote_config_dir().join(&format!("{}.json", name))
    }

    pub fn get_remote_config_dir(&self) -> PathBuf {
        Path::new(&self.config_dir).join(TOOL_REMOTE_DIR)
    }

    pub fn get_remote_download_dir(&self) -> PathBuf {
        Path::new(&self.config_dir).join(TOOL_DOWNLOAD_DIR)
    }

    pub fn get_link_dir(&self) -> PathBuf {
        Path::new(&self.tool_root_dir).join(TOOL_LINK_FOLDER_NAME)
    }

    pub fn shim_from_env() -> Self {
        let project_dirs =
            ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
        let log_dir = project_dirs.data_dir().display().to_string();

        let tool_root_dir = match std::env::var(TOOLUP_ROOT_TOOL_DIR) {
            Ok(config_dir) => config_dir,
            Err(_) => Path::join(project_dirs.cache_dir(), "download")
                .display()
                .to_string(),
        };

        let config_dir = match std::env::var(TOOLUP_GLOBAL_CONFIG_DIR) {
            Ok(config_dir) => config_dir,
            Err(_) => project_dirs.config_dir().to_str().unwrap().to_owned(),
        };

        Self {
            log_dir,
            config_dir,
            tool_root_dir,
        }
    }
}

impl From<&GlobalConfig> for GlobalFolders {
    fn from(cli: &GlobalConfig) -> Self {
        let project_dirs =
            ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
        let log_dir = project_dirs.data_dir().display().to_string();

        let tool_root_dir = match &cli.tool_root_dir {
            Some(config_dir) => config_dir.to_string(),
            None => Path::join(project_dirs.cache_dir(), "download")
                .display()
                .to_string(),
        };

        let config_dir = match &cli.config_dir {
            Some(config_dir) => config_dir.to_string(),
            None => project_dirs.config_dir().to_str().unwrap().to_owned(),
        };

        Self {
            log_dir,
            config_dir,
            tool_root_dir,
        }
    }
}

impl LoggingOpts {
    pub fn to_level(&self) -> LevelFilter {
        if self.error {
            LevelFilter::ERROR
        } else if self.warn {
            LevelFilter::WARN
        } else if self.debug == 0 {
            LevelFilter::INFO
        } else if self.debug == 1 {
            LevelFilter::DEBUG
        } else {
            LevelFilter::TRACE
        }
    }
}

pub fn get_hash_for_contents(input: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();

    format!("{:x}", result)
}

#[cfg(target_family = "unix")]
pub fn create_link<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
) -> Result<(), std::io::Error> {
    debug!("Creating symlink");
    std::os::unix::fs::symlink(original, link)
}

#[cfg(target_family = "windows")]
pub fn create_link<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
) -> Result<(), std::io::Error> {
    std::os::windows::fs::symlink_file(original, link)
}

#[cfg(target_family = "windows")]
pub fn set_executable(_path: &PathBuf) {}

#[cfg(target_family = "unix")]
pub fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    debug!("Setting {:?} as executable", path);

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o744);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(target_family = "windows")]
pub fn set_executable(_path: &PathBuf) {}

#[cfg(target_family = "unix")]
pub fn exec(path: String, args: Vec<String>) {
    use std::ffi::CString;

    let path = CString::new(path).unwrap();
    let argv: Vec<CString> = args.into_iter().map(|x| CString::new(x).unwrap()).collect();
    nix::unistd::execv(&path, argv.as_slice()).unwrap();
}

#[cfg(target_family = "windows")]
pub fn exec(path: String, args: Vec<String>) {
    use std::process::{self, Command};
    let status = Command::new("cmd")
        .arg("/C")
        .arg(path)
        .arg(args.join(" "))
        .status()
        .unwrap();

    process::exit(status.code().unwrap_or(0));
}

pub fn extract_env_from_script(
    script: &crate::model::AuthScript,
) -> Result<BTreeMap<String, String>, std::io::Error> {
    use std::io::BufRead;
    let mut extracted = BTreeMap::new();

    let mut command = Command::new(script.script_path.to_string());
    let output = command.output()?;

    for line in output.stdout.lines() {
        let line = line?;
        match line.replace("export ", "").split_once("=") {
            Some((left, right)) => {
                extracted.insert(left.to_string(), right.to_string());
            }
            None => debug!("Unable to parse {:?}", line),
        }
    }

    Ok(extracted)
}

#[test]
fn validate_extract() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let stub_auth = format!("{}/test/stub-auth.sh", manifest_dir);
    let extracted = extract_env_from_script(&crate::model::AuthScript {
        script_path: stub_auth,
    })
    .unwrap();

    assert_eq!("bar", extracted.get("foo").unwrap());
    assert_eq!("foo", extracted.get("bar").unwrap());
}

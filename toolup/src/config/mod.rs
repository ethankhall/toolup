pub mod model;
pub mod parse;
pub mod lock;

use std::default::Default;
use std::path::{Path, PathBuf};
use std::fs;

use clap::ArgMatches;
use toml;

use crate::*;
use self::model::*;
use crate::common::error::*;

type Result<T> = std::result::Result<T, CliError>;

#[derive(Clone, Debug)]
pub struct ConfigContainer {
    pub lock_config_path: PathBuf,
    pub lock_config: Option<lock::ToolLock>,
}

impl ConfigContainer {
    pub fn default_lock_config_path() -> PathBuf {
        let toolup_config_dir = Path::new(CONFIG_DIR.as_str());
        toolup_config_dir.join(Path::new("toolup.lock")).to_path_buf()
    }

    pub fn set_lock_config(tool_lock: lock::ToolLock) {
        let lock_config_path = ConfigContainer::get_container_config().lock_config_path.clone();

        let new_container = ConfigContainer {
            lock_config_path: lock_config_path,
            lock_config: Some(tool_lock)
        };

        *crate::CONFIG_DATA.write().unwrap() = Box::new(new_container);
    }

    pub fn get_container_config() -> Box<ConfigContainer> {
        crate::CONFIG_DATA.read().unwrap().clone()
    }

    pub fn write_config() -> Result<()> {
        let config = Self::get_container_config();

        if !config.lock_config_path.parent().unwrap().exists() {
            fs::create_dir_all(config.lock_config_path.parent().unwrap())?;
        }

        debug!("Writing lock to {:#?}.", &config.lock_config_path);

        let text = match serde_json::to_string_pretty(&config.lock_config) {
            Ok(text) => text,
            Err(err) => {
                warn!("Unable to seralize state file.");
                err!(ConfigError::UnableToWriteConfig(err.to_string()))
            }
        };

        match fs::write(config.lock_config_path, text) {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("Unable to write state file.");
                err!(ConfigError::UnableToWriteConfig(e.to_string()))
            }
        }
    }
}

impl Default for ConfigContainer {
    fn default() -> Self {
        ConfigContainer {
            lock_config_path: ConfigContainer::default_lock_config_path(),
            lock_config: None
        }
    }
}

pub use parse::{parse_config, initialize_configs};
pub use model::*;
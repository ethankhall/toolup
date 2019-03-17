use std::fs;
use std::path::PathBuf;

use clap::ArgMatches;

use crate::common::error::*;
use crate::config::lock::*;
use crate::*;

pub fn initialize_configs(args: &ArgMatches) {
    let lock_config_path = match args.value_of("lock") {
        Some(config_file) => PathBuf::from(config_file),
        None => ConfigContainer::default_lock_config_path(),
    };

    let config_container = ConfigContainer {
        lock_config_path,
        lock_config: None,
    };

    *crate::CONFIG_DATA.write().unwrap() = Box::new(config_container);
}

pub fn parse_config(args: &ArgMatches) -> Result<(), CliError> {
    let lock_config_path = ConfigContainer::get_container_config()
        .lock_config_path
        .clone();
    let lock_config = read_existing_lock(&lock_config_path, args)?;

    ConfigContainer::set_lock_config(lock_config);

    Ok(())
}

fn read_existing_lock(global_path: &PathBuf, args: &ArgMatches) -> Result<ToolLock, CliError> {
    if !global_path.exists() {
        err!(ConfigError::ConfigFileNotFound(global_path.to_path_buf()))
    }

    trace!("Reading config from {:#?}", global_path);

    let lock_file = match fs::read_to_string(&global_path) {
        Ok(contents) => match serde_json::from_str::<ToolLock>(&contents) {
            Ok(mut config) => {
                if let Some(value) = args.value_of("github_api_token") {
                    config.tokens.github = Some(s!(value))
                }
                config
            }
            Err(_) => err!(ConfigError::ConfigFormatError(s!(
                "Unable to deserialize existing state file."
            ))),
        },
        Err(_) => err!(ConfigError::ConfigFormatError(s!(
            "Unable to deserialize existing state file."
        ))),
    };

    Ok(lock_file)
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_new_and_add() {
        let mut lock = ToolLock::default();

        let now: DateTime<Utc> = Utc::now();

        let tool_version = ToolVersion {
            name: s!("foo"),
            version: s!("bar"),
            created_at: now,
            download_url: Some(s!("http://localhost/help")),
            exec_path: s!("foo.exe"),
            art_type: ArtifactType::Zip,
            auth_token_source: AuthTokenSource::None,
        };

        lock.add_new(tool_version);
        let tool_lock = serde_json::to_string(&lock).unwrap();

        serde_json::from_str::<ToolLock>(&tool_lock).unwrap();
    }
}

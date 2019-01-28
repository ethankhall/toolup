use std::path::PathBuf;
use std::fs;

use clap::ArgMatches;
use toml;

use super::model::*;
use super::error::*;

type Result<T> = std::result::Result<T, CliError>;

pub fn parse_config(path: PathBuf, args: &ArgMatches) -> Result<GlobalConfig> {
    if !path.exists() {
        err!(ConfigError::ConfigFileNotFound(path))
    }

    let contents: String = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) => err!(IOError::UnableToReadFile(path.clone(), err.to_string()))
    };

    debug!("Read contents from {:?}", path);

    let mut config = match toml::from_str::<GlobalConfig>(&contents) {
        Ok(config) => config,
        Err(err) => err!(ConfigError::ConfigFormatError(err.to_string()))
    };

    if let Some(value) = args.value_of("github_api_token") {
        config.tokens.github = Some(s!(value))
    }

    Ok(config)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    use super::*;

    #[test]
    fn test_configs() {
        let config: GlobalConfig = assert_ok!(verify_config_parses(s!("example-1.toml")));
        let crom: &ApplicationConfig = config.tools().get("crom").unwrap();
        assert_eq!(UpdateFrequency::Fast, crom.update_frequency);
    }

    fn verify_config_parses(filename: String) -> Result<GlobalConfig> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("samples");
        path.push(filename);

        println!("path: {:?}", path);
        parse_config(path, &ArgMatches::default())      
    }
}
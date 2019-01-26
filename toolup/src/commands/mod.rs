use std::path::{Path, PathBuf};

use clap::ArgMatches;
use directories::ProjectDirs;

use crate::common::error::*;
use crate::common::config::*;
use crate::common::model::*;
use crate::err;

pub type CliResult = Result<i32, CliError>;

fn get_config(args: &ArgMatches) -> Result<GlobalConfig, CliError> {
    let config_file = match args.value_of("config") {
        Some(config_file) => PathBuf::from(config_file),
        None => {
            let project_dirs = ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
            let toolup_config_dir = project_dirs.config_dir();

            toolup_config_dir.join(Path::new("toolup.toml")).to_path_buf()
        }
    };
    
    parse_config(config_file)
}

pub fn run_show_version(args: &ArgMatches) -> CliResult { 
    let config = get_config(args)?;
    
    let tool_names: Vec<String> = if args.is_present("all") {
        config.tools.keys().map(|x| s!(x)).collect()
    } else {
        vec!(s!(args.value_of("TOOL").unwrap()))
    };

    err!(()) 
}

pub fn run_lock_tool(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_unlock_tool(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_status(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_update(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_exec(_args: &ArgMatches) -> CliResult { err!(()) }
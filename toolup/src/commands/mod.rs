use std::path::{Path, PathBuf};

use clap::ArgMatches;
use directories::ProjectDirs;

use crate::common::error::*;
use crate::common::config::*;
use crate::common::model::*;
use crate::err;

use crate::version::*;

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
    
    parse_config(config_file, &args)
}

pub fn run_show_version(args: &ArgMatches) -> CliResult { 
    let config = get_config(args)?;
    
    let tool_names: Vec<String> = if args.is_present("all") {
        config.tools.keys().map(|x| s!(x)).collect()
    } else {
        vec!(s!(args.value_of("NAME").unwrap()))
    };

    let include_missing = args.is_present("include_missing");

    for tool_name in tool_names {
        let tool: &ApplicationConfig = match config.tools.get(&tool_name) {
            Some(tool) => tool,
            None => err!(ConfigError::ToolNotFound(tool_name))
        };

        let version_api = get_version_api(&tool.version_source, &config.tokens, tool.artifact.get_name())?;

        let latest_versions = version_api.get_versions(15)?;
        println!("Tool: {}", tool_name);
        
        let lines: Vec<String> = latest_versions
            .iter()
            .filter(|x| include_missing || x.artifact_path.is_some())
            .map(|version| {
                match version.artifact_path {
                    Some(_) => s!(version.name),
                    None => s!(format!("{} - Artifact missing", version.name))
                }
            })
            .collect();

        for (i, line) in lines.iter().enumerate() {
            if i + 1 == lines.len() {
                println!("  \u{2514}\u{2500} {}", line);
            } else {
                println!("  \u{251C}\u{2500} {}", line);
            }
        }
    }

    err!(()) 
}

pub fn run_lock_tool(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_unlock_tool(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_status(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_update(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_exec(_args: &ArgMatches) -> CliResult { err!(()) }
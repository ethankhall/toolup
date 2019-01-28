use std::path::{Path, PathBuf};

use clap::ArgMatches;
use directories::ProjectDirs;

use crate::common::error::*;
use crate::common::config::*;
use crate::common::model::*;
use crate::err;

use crate::storage::get_global_state;
use crate::storage::model::*;

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
        config.tools().keys().map(|x| s!(x)).collect()
    } else {
        vec!(s!(args.value_of("NAME").unwrap()))
    };

    let include_missing = args.is_present("include_missing");
    let global_state = get_global_state(&config)?;

    for tool_name in tool_names {
        let tool: &ToolGlobalState = match global_state.tools.get(&tool_name) {
            Some(tool) => tool,
            None => err!(ConfigError::ToolNotFound(tool_name))
        };

        println!("Tool: {}", tool_name);

        let versions = tool.get_versions();

        let mut lines: Vec<String> = Vec::new();

        for version in versions { 
            let v = tool.get_version(&version).unwrap();
            match (include_missing, v) {
                (true, ToolVersion::NoArtifact { name }) => lines.push(s!(format!("{} - Artifact missing", name))),
                (_, ToolVersion::Artifact { name, download_url: _, installed: _ }) => lines.push(s!(format!("{}", name))),
                _ => {}
            }
        }

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
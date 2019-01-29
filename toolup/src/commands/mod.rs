use std::path::{Path, PathBuf};

use clap::ArgMatches;
use directories::ProjectDirs;

use crate::common::error::*;
use crate::common::config::*;
use crate::common::model::*;
use crate::err;

use crate::storage::{download_tools, get_global_state};
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

        let mut versions: Vec<ToolVersion> = Vec::new();
        versions.extend(tool.get_all_versions().into_iter());

        let mut lines: Vec<String> = Vec::new();
        versions.sort_by(|a, b| b.created_at().cmp(&a.created_at()));

        for version in versions {
            match (include_missing, version) {
                (true, ToolVersion::NoArtifact(no_art)) => lines.push(s!(format!("{} - Artifact missing", no_art.name))),
                (_, ToolVersion::Artifact(art)) => lines.push(s!(format!("{}", art.name))),
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

    Ok(0)
}

pub fn run_lock_tool(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_unlock_tool(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_status(_args: &ArgMatches) -> CliResult { err!(()) }

pub fn run_update(args: &ArgMatches) -> CliResult { 
    let config = get_config(args)?;
    let global_state = get_global_state(&config)?;

    let tool_names: Vec<String> = config.tools().keys().map(|x| s!(x)).collect();
    
    match download_tools(&global_state, tool_names) {
        Ok(_) => Ok(0),
        Err(e) => Err(e)
    }
}

pub fn run_exec(_args: &ArgMatches) -> CliResult { err!(()) }
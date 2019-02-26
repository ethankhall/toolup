use std::collections::{BinaryHeap, BTreeMap};

use clap::ArgMatches;

use crate::common::error::*;
use crate::common::config::*;
use crate::err;

use crate::ConfigFiles;
use crate::storage::{download_tools, update_global_state};
use crate::storage::lock::*;
use crate::storage::link::*;

pub type CliResult = Result<i32, CliError>;

fn get_global_config(config_file: &ConfigFiles, args: &ArgMatches) -> Result<ToolLock, CliError> {
    match read_existing_lock(config_file.lock_path.clone()) {
        Some(config) => Ok(config),
        None => {
            let glocal_config = parse_config(config_file.config_path.clone(), args)?;
            update_global_state(ToolLock::default(), &glocal_config)
        }
    }
}

pub fn run_show_version(config_file: &ConfigFiles, args: &ArgMatches) -> CliResult { 
    let lock = get_global_config(config_file, args)?;
    
    let tool_list: Vec<ToolVersion> = if args.is_present("all") {
        lock.get_all_tools()
    } else {
        lock.find_tool(&args.value_of("NAME").unwrap())
    };

    let mut tool_map: BTreeMap<String, BinaryHeap<ToolVersion>> = BTreeMap::new();
    for tool in tool_list {
        let tool_list = tool_map.entry(tool.name.clone()).or_insert_with(|| BinaryHeap::new() );

        tool_list.push(tool);
    }

    let include_missing = args.is_present("include_missing");

    info!("Lock file located at {}", config_file.lock_path.to_str().unwrap());
    info!("Config file located at {}", config_file.config_path.to_str().unwrap());

    for (tool_name, versions) in tool_map.iter() {
        info!("Tool: {}", tool_name);

        let mut lines: Vec<String> = Vec::new();

        for version in versions.iter() {
            if version.is_downloadable() {
                let line = if version.artifact_exists() {
                    format!("{} - {} - Installed", version.name, version.version)
                } else {
                    format!("{} - {}", version.name, version.version)
                };

                lines.push(line);
            } else if include_missing {
                lines.push(format!("{} - {} - Artifact not avaliable", version.name, version.version));
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

pub fn run_update(config_file: &ConfigFiles, args: &ArgMatches) -> CliResult { 
    let lock = get_global_config(config_file, args)?;

    let wanted_versions: Vec<ToolVersion> = lock.get_all_wanted();

    if let Err(e) = download_tools(&lock, &wanted_versions) {
        return Err(e);
    }

    match update_links(&wanted_versions) {
        Ok(_) => Ok(0),
        Err(e) => Err(e)
    }
}

pub fn run_exec(_args: &ArgMatches) -> CliResult { err!(()) }
pub mod manage;

use std::collections::{BTreeMap, BinaryHeap};

use clap::ArgMatches;

use crate::common::error::*;
use crate::err;

use crate::config::lock::*;
use crate::config::ConfigContainer;
use crate::storage::link::*;
use crate::storage::{download_tools, pull_for_latest};

pub type CliResult = Result<i32, CliError>;

pub fn run_show_version(args: &ArgMatches) -> CliResult {
    let lock = ToolLock::get_global_lock();

    let tool_list: Vec<ToolVersion> = if args.is_present("all") {
        lock.get_all_tools()
    } else {
        lock.find_tool(&args.value_of("NAME").unwrap())
    };

    let mut tool_map: BTreeMap<String, BinaryHeap<ToolVersion>> = BTreeMap::new();
    for tool in tool_list {
        let tool_list = tool_map
            .entry(tool.name.clone())
            .or_insert_with(|| BinaryHeap::new());

        tool_list.push(tool);
    }

    let include_missing = args.is_present("include_missing");
    print_current_state(tool_map, include_missing);

    Ok(0)
}

pub fn run_lock_tool(_args: &ArgMatches) -> CliResult {
    err!(())
}

pub fn run_unlock_tool(_args: &ArgMatches) -> CliResult {
    err!(())
}

pub fn run_status(_args: &ArgMatches) -> CliResult {
    let lock = ToolLock::get_global_lock();
    let tool_list: Vec<ToolVersion> = lock.get_all_tools();

    let mut tool_map: BTreeMap<String, BinaryHeap<ToolVersion>> = BTreeMap::new();
    for tool in tool_list {
        let tool_list = tool_map
            .entry(tool.name.clone())
            .or_insert_with(|| BinaryHeap::new());

        tool_list.push(tool);
    }

    print_current_state(tool_map, true);

    Ok(0)
}

pub fn run_update(_args: &ArgMatches) -> CliResult {
    pull_for_latest()?;

    let lock = ToolLock::get_global_lock();

    let wanted_versions: Vec<ToolVersion> = lock.get_all_wanted();
    trace!("Wanted versions: {:?}", wanted_versions);

    if let Err(e) = download_tools(&lock, &wanted_versions) {
        return Err(e);
    }

    match update_links(&wanted_versions) {
        Ok(_) => Ok(0),
        Err(e) => Err(e),
    }
}

pub fn run_exec(args: &ArgMatches) -> CliResult {
    let lock = ToolLock::get_global_lock();

    let tool = lock.get_wanted(args.value_of("TOOL").unwrap());
    if let Some(tool) = tool {
        let path = tool.exec_path();
        let arg_path: Vec<String> = args
            .values_of("ARGS")
            .unwrap_or_default()
            .map(|x| s!(x))
            .collect();
        exec(s!(path.to_str().unwrap()), arg_path);
        return Ok(0);
    }

    err!(ConfigError::ToolNotFound(s!(args
        .value_of("TOOL")
        .unwrap())))
}

#[cfg(target_family = "unix")]
fn exec(path: String, args: Vec<String>) {
    use std::ffi::CString;

    let path = CString::new(path).unwrap();
    let argv: Vec<CString> = args.into_iter().map(|x| CString::new(x).unwrap()).collect();
    nix::unistd::execv(&path, argv.as_slice()).unwrap();
}

#[cfg(target_family = "windows")]
fn exec(path: String, args: Vec<String>) {
    use std::process::{self, Command};
    let status = Command::new("cmd")
        .arg("/C")
        .arg(path)
        .arg(args.join(" "))
        .status()
        .unwrap();

    process::exit(status.code().unwrap_or(0));
}

fn print_current_state(tool_map: BTreeMap<String, BinaryHeap<ToolVersion>>, include_missing: bool) {
    info!(
        "Lock file located at {:?}",
        ConfigContainer::get_container_config().lock_config_path
    );

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
                lines.push(format!(
                    "{} - {} - Artifact not avaliable",
                    version.name, version.version
                ));
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
}

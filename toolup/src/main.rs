#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate serde;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate toolup_macros;
#[macro_use]
extern crate kopy_common_lib;
extern crate directories;
extern crate json;
extern crate indicatif;
extern crate chrono;
extern crate http;
extern crate tar;
extern crate zip;
extern crate atty;
#[macro_use]
extern crate lazy_static;

mod storage;
mod common;
mod commands;

use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use clap::{App, ArgMatches};

lazy_static! {
    pub static ref CONFIG_DIR: String = {
        let project_dirs = ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
        s!(project_dirs.cache_dir().to_str().unwrap())
    };

    pub static ref CACHE_DIR: String = {
        let project_dirs = ProjectDirs::from("io", "ehdev", "toolup.cache").expect("To create project dirs");
        s!(project_dirs.cache_dir().to_str().unwrap())
    };

    pub static ref PATH_DIR: String = {
        let project_dirs = ProjectDirs::from("io", "ehdev", "toolup.bin").expect("To create project dirs");
        s!(project_dirs.cache_dir().to_str().unwrap())
    };
}

pub struct ConfigFiles {
    pub lock_path: PathBuf,
    pub config_path: PathBuf
}

fn find_config_files(args: &ArgMatches) -> ConfigFiles {
    let config_path = match args.value_of("config") {
        Some(config_file) => PathBuf::from(config_file),
        None => {
            let toolup_config_dir = Path::new(CONFIG_DIR.as_str());

            toolup_config_dir.join(Path::new("toolup.toml")).to_path_buf()
        }
    };

    let lock_path = match args.value_of("lock") {
        Some(config_file) => PathBuf::from(config_file),
        None => {
            let toolup_config_dir = Path::new(CONFIG_DIR.as_str());
            toolup_config_dir.join(Path::new("toolup.lock")).to_path_buf()
        }
    };
    
    ConfigFiles { lock_path, config_path }
}

fn main() {
    let yml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yml)
        .version(&*format!("v{}", crate_version!()))
        .get_matches();

    kopy_common_lib::configure_logging(
        matches.occurrences_of("debug") as i32,
        matches.is_present("warn"),
        matches.is_present("quite"),
    );

    let config_files = find_config_files(&matches);

    let command = match matches.subcommand() {
        ("path", Some(_)) => { println!("{}", s!(PATH_DIR)); Ok(0) }
        ("show-version", Some(cmd_match)) => commands::run_show_version(&config_files, cmd_match),
        ("lock-tool", Some(cmd_match)) => commands::run_lock_tool(cmd_match),
        ("unlock-tool", Some(cmd_match)) => commands::run_unlock_tool(cmd_match),
        ("status", Some(cmd_match)) => commands::run_status(cmd_match),
        ("update", Some(cmd_match)) => commands::run_update(&config_files, cmd_match),
        ("run", Some(cmd_match)) => commands::run_exec(cmd_match),
        _ => { panic!("This is a bug, please report the command wasn't found.")}
    };

    match command {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("Error while running toolup: {}", err);
            std::process::exit(err.into())
        }
    }
}

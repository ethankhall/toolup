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

mod storage;
mod common;
mod commands;

use clap::App;

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

    let command = match matches.subcommand() {
        ("show-version", Some(cmd_match)) => commands::run_show_version(cmd_match),
        ("lock-tool", Some(cmd_match)) => commands::run_lock_tool(cmd_match),
        ("unlock-tool", Some(cmd_match)) => commands::run_unlock_tool(cmd_match),
        ("status", Some(cmd_match)) => commands::run_status(cmd_match),
        ("update", Some(cmd_match)) => commands::run_update(cmd_match),
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

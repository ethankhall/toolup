#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate serde;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate toolup_macros;

#[macro_export]
macro_rules! s {
    ($x:expr) => {
        $x.to_string()
    };
}

mod common;

use clap::App;

fn main() {
    let yml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yml)
        .version(&*format!("v{}", crate_version!()))
        .get_matches();

    match matches.subcommand() {
        ("show-version", Some(cmd_match)) => unimplemented!(),
        ("lock-tool", Some(cmd_match)) => unimplemented!(),
        ("unlock-tool", Some(cmd_match)) => unimplemented!(),
        ("status", Some(cmd_match)) => unimplemented!(),
        ("update", Some(cmd_match)) => unimplemented!(),
        ("run", Some(cmd_match)) => unimplemented!(),
        _ => { panic!("This is a bug, please report the command wasn't found.")}
    }
}

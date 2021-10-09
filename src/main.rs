use std::default::Default;

use anyhow::Result as AnyResult;
use clap::{ArgGroup, AppSettings, Clap};
use tracing::{error, debug, level_filters::LevelFilter};
use tracing_subscriber::{filter::{filter_fn}, prelude::*};
use tracing_subscriber::{
    fmt::format::{Format, JsonFields, PrettyFields},
    layer::SubscriberExt,
    Registry,
};

mod commands;
mod model;
mod util;

use commands::{InitToolSubCommand, ArchiveToolSubCommand, InstallToolSubCommand, handle_package};

#[derive(Clap, Debug)]
#[clap(author, version)]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(flatten)]
    pub logging_opts: LoggingOpts,

    #[clap(subcommand)]
    pub sub_command: SubCommand,
}

#[derive(Clap, Debug)]
pub enum SubCommand {
    
    /// Manage toolup managed commands
    #[clap(subcommand)]
    Manage(MangeSubCommand),

    /// Manage packages locally.
    #[clap(subcommand)]
    Package(PackageSubCommand)
}

#[derive(Clap, Debug)]
#[clap(setting = AppSettings::ColoredHelp)]
pub enum PackageSubCommand {
    /// Create an empty config file intended to be updated by user.
    Init(InitToolSubCommand),
    /// Archive a package based on configuration file.
    Archive(ArchiveToolSubCommand),
    /// Install a local package archive
    Install(InstallToolSubCommand),
}

#[derive(Clap, Debug)]
#[clap(setting = AppSettings::ColoredHelp)]
pub enum MangeSubCommand {
    /// Add a remote tool configuration
    Add(ToolAddArgs),
    /// Delete a remote tool configuration
    Delete(ToolAddArgs),
    /// Fetch a remote tool
    Fetch(ToolAddArgs),
}

#[derive(Clap, Debug)]
pub struct ToolAddArgs {
    
}

#[derive(Clap, Debug)]
#[clap(group = ArgGroup::new("logging"))]
pub struct LoggingOpts {
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences), global(true), group = "logging")]
    pub debug: u64,

    /// Enable warn logging
    #[clap(short, long, global(true), group = "logging")]
    pub warn: bool,

    /// Disable everything but error logging
    #[clap(short, long, global(true), group = "logging")]
    pub error: bool,

    /// When set, logs will be written to stdout in addtion to the file.
    #[clap(short, long, global(true))]
    pub console: bool,
}

impl LoggingOpts {
    pub fn to_level(&self) -> LevelFilter {
        if self.error {
            LevelFilter::ERROR
        } else if self.warn {
            LevelFilter::WARN
        } else if self.debug == 0 {
            LevelFilter::INFO
        } else if self.debug == 1 {
            LevelFilter::DEBUG
        } else {
            LevelFilter::TRACE
        }
    }
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    dotenv::dotenv().ok();
    human_panic::setup_panic!();

    let opt = Opts::parse();

    let _gaurd = configure_logging(&opt.logging_opts);

    debug!("Starting Execution");

    let result = match opt.sub_command {
        SubCommand::Package(args) => handle_package(args).await,
        _ => unimplemented!()
    };

    if let Err(e) = result {
        eprint!("Failed to execute command: {}", e);
        std::process::exit(1);
    }

    // match opt.sub_command {
    //     SubCommand::Init => run_init().await,
    //     SubCommand::RunMigration(args) => run_migration(args).await,
    //     SubCommand::CheckStatus(args) => check_status(args).await,
    //     SubCommand::RunFollowup(args) => run_followup(args).await,
    // }

    Ok(())
}

fn configure_logging(logging_opts: &LoggingOpts) -> tracing_appender::non_blocking::WorkerGuard {
    

    let file_appender = tracing_appender::rolling::hourly(util::LOG_DIR.to_string(), "toolup.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_output = tracing_subscriber::fmt::layer()
        .event_format(Format::default().json().flatten_event(true))
        .fmt_fields(JsonFields::new())
        .with_writer(non_blocking);

    let console_output = tracing_subscriber::fmt::layer()
            .event_format(Format::default().pretty())
            .fmt_fields(PrettyFields::new())
            .with_target(false);

    let override_console_output = tracing_subscriber::fmt::layer()
            .event_format(Format::default().pretty())
            .fmt_fields(PrettyFields::new())
            .with_target(false);


    let enable_stdout = logging_opts.console;

    let subscriber = Registry::default()
        .with(logging_opts.to_level())
        .with(console_output.with_filter(filter_fn(move |metadata| {
            !enable_stdout && metadata.target() == "user"
        })));

    let enable_stdout = logging_opts.console;
    let subscriber = subscriber
        .with(override_console_output.with_filter(filter_fn(move |_metadata| {
            enable_stdout
        })))
        .with(file_output);


    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    guard
}
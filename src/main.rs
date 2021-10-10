use std::default::Default;

use anyhow::Result as AnyResult;
use clap::Clap;
use tracing::{debug, error, level_filters::LevelFilter};
use tracing_subscriber::{filter::filter_fn, prelude::*};
use tracing_subscriber::{
    fmt::format::{Format, JsonFields, PrettyFields},
    layer::SubscriberExt,
    Registry,
};

mod cli;
mod commands;
mod model;
mod state;
mod util;

use cli::*;
use commands::handle_package;

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
        _ => unimplemented!(),
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

    let subscriber =
        Registry::default()
            .with(logging_opts.to_level())
            .with(console_output.with_filter(filter_fn(move |metadata| {
                !enable_stdout && metadata.target() == "user"
            })));

    let enable_stdout = logging_opts.console;
    let subscriber = subscriber
        .with(override_console_output.with_filter(filter_fn(move |_metadata| enable_stdout)))
        .with(file_output);

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    guard
}

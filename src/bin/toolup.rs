use std::default::Default;

use anyhow::Result as AnyResult;
use clap::Clap;
use tracing::{debug, error};
use tracing_subscriber::{filter::filter_fn, prelude::*};
use tracing_subscriber::{
    fmt::format::{Format, JsonFields, PrettyFields},
    layer::SubscriberExt,
    Registry,
};

use toolup::prelude::*;

#[tokio::main]
async fn main() -> AnyResult<()> {
    dotenv::dotenv().ok();
    human_panic::setup_panic!();

    let opt = Opts::parse();
    let global_folder = GlobalFolders::from(&opt.global_config);

    let _gaurd = configure_logging(&opt.logging_opts, &global_folder);

    debug!("Starting Execution");

    let result = run_command(opt, &global_folder).await;

    if let Err(e) = result {
        error!(target: "user", "Failed to execute command: {}", e);
        drop(_gaurd);
        std::process::exit(1);
    }

    Ok(())
}

async fn run_command(opts: Opts, global_folder: &GlobalFolders) -> Result<(), CommandError> {
    let result = match opts.sub_command {
        SubCommand::Package(args) => handle_package(args, &global_folder).await?,
        SubCommand::Exec(args) => handle_exec(args, &global_folder).await?,
        _ => unimplemented!(),
    };

    Ok(result)
}

fn configure_logging(
    logging_opts: &LoggingOpts,
    global_folder: &GlobalFolders,
) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender =
        tracing_appender::rolling::hourly(global_folder.log_dir.clone(), "toolup.log");
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

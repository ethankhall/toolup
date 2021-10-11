use clap::{AppSettings, ArgGroup, Clap};

#[derive(Clap, Debug)]
#[clap(author, version)]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(flatten)]
    pub logging_opts: LoggingOpts,

    #[clap(subcommand)]
    pub sub_command: SubCommand,

    #[clap(flatten)]
    pub global_config: GlobalConfig,
}

#[derive(Clap, Debug)]
pub enum SubCommand {
    /// Manage toolup managed commands
    #[clap(subcommand)]
    Manage(MangeSubCommand),

    /// Manage packages locally.
    #[clap(subcommand)]
    Package(PackageSubCommand),

    /// Exec one of the installed packages
    Exec(ExecSubCommand),
}

#[derive(Clap, Debug)]
pub struct ExecSubCommand {
    /// Use a specific version of the binary, not the current one.
    #[clap(long, env = "TOOLUP_VERSION_OVERRIDE")]
    pub version: Option<String>,
    /// Name of the command to execute
    pub command_name: String,
    /// Arguments to be passed to command.
    pub args: Vec<String>,
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
pub struct ArchiveToolSubCommand {
    /// Location on disk that has the artifact directory ready.
    ///
    /// All files relative to this directory will be packaged up for distribution.
    /// There is a limit of 128 MiB total uncompressed files.
    #[clap(long)]
    pub target_dir: String,

    /// The config file that describes the tool that is being packaged.
    #[clap(long = "config")]
    pub application_config: String,

    /// Directory to write the archive to. The final file will be named `{name}-{version}.tar.gz`.
    #[clap(long)]
    pub archive_dir: String,
}

#[derive(Clap, Debug)]
pub struct InitToolSubCommand {
    #[clap(default_value("package.toml"))]
    pub output_file: String,
}

#[derive(Clap, Debug)]
pub struct InstallToolSubCommand {
    /// Location on disk has the pre-built package.
    ///
    /// This package will be extracted, and placed inside your user directory.
    /// When a package is isntalled locally, it will no longer support refreshs
    /// from an upstream source.
    #[clap(long)]
    pub archive_path: String,

    /// If the package already exists, overwrite it.
    ///
    /// When set, toolup will clearn out the destination directory if it exists.
    #[clap(long)]
    pub overwrite: bool,
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
pub struct ToolAddArgs {}

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

#[derive(Clap, Debug)]
pub struct GlobalConfig {
    /// A level of verbosity, and can be used multiple times
    #[clap(long, global(true), env(crate::util::TOOLUP_GLOBAL_CONFIG_DIR))]
    pub config_dir: Option<String>,

    /// Override the location to install the package.
    ///
    /// This option will allow you to install the package in a custom directory,
    /// instead of the default one managed by toolup.
    #[clap(long, global(true), env(crate::util::TOOLUP_ROOT_TOOL_DIR))]
    pub tool_root_dir: Option<String>,
}

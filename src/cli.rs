use clap::{ArgGroup, ColorChoice, Parser, ArgEnum};

#[derive(Parser, Debug)]
#[clap(author, version, color = ColorChoice::Always)]
pub struct Opts {
    #[clap(flatten)]
    pub logging_opts: LoggingOpts,

    #[clap(subcommand)]
    pub sub_command: SubCommand,

    #[clap(flatten)]
    pub global_config: GlobalConfig,
}

#[derive(Parser, Debug)]
pub enum SubCommand {
    /// Manage toolup managed commands
    #[clap(subcommand)]
    Remote(RemoteSubCommand),

    /// Manage packages locally.
    #[clap(subcommand)]
    Package(PackageSubCommand),

    /// Exec one of the installed packages
    Exec(ExecSubCommand),

    /// Get config details
    #[clap(subcommand)]
    Config(ConfigSubCommand),

    /// Display version info about toolup
    Version,
}

#[derive(Parser, Debug)]
#[clap(color = ColorChoice::Always)]
pub enum ConfigSubCommand {
    /// Print the path to the binary link path
    GetLinkPath(GetPathSubCommand),
}

#[derive(Parser, Debug)]
pub struct GetPathSubCommand {}

#[derive(Parser, Debug)]
pub struct ExecSubCommand {
    /// Use a specific version of the binary, not the current one.
    #[clap(long, env = "TOOLUP_VERSION_OVERRIDE")]
    pub version: Option<String>,
    /// Name of the command to execute
    pub command_name: String,
    /// Arguments to be passed to command.
    pub args: Vec<String>,
}

#[derive(Parser, Debug)]
#[clap(color = ColorChoice::Always)]
pub enum PackageSubCommand {
    /// Create an empty config file intended to be updated by user.
    Init(InitToolSubCommand),
    /// Archive a package based on configuration file.
    Archive(ArchiveToolSubCommand),
    /// Install a local package archive
    Install(InstallToolSubCommand),
}

#[derive(Parser, Debug)]
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

#[derive(Parser, Debug)]
pub struct InitToolSubCommand {
    #[clap(default_value("package.toml"))]
    pub output_file: String,
}

#[derive(Parser, Debug)]
pub struct InstallToolSubCommand {
    /// Location on disk has the pre-built package.
    ///
    /// This package will be extracted, and placed inside your user directory.
    /// When a package is isntalled locally, it will no longer support refreshs
    /// from an upstream source.
    pub archive_path: String,

    /// If the package already exists, overwrite it.
    ///
    /// When set, toolup will clearn out the destination directory if it exists.
    #[clap(long)]
    pub overwrite: bool,
}

#[derive(Parser, Debug)]
#[clap(color = ColorChoice::Always)]
pub enum RemoteSubCommand {
    /// Add a remote tool configuration
    #[clap(subcommand)]
    Add(AddRemoteSubCommand),
    /// Delete a remote tool configuration
    Delete(DeleteRemoteSubCommand),
    /// List all the installed remotes
    List(ListRemoteSubCommand),
    /// Update one/many remote tool
    Update(UpdateRemoteSubCommand),
}

#[derive(Parser, Debug)]
pub struct UpdateRemoteSubCommand {
    /// When specified, only the remote matching the name provided will be updated.
    #[clap(long)]
    pub only: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ListRemoteSubCommand {}

#[derive(Parser, Debug)]
pub struct DeleteRemoteSubCommand {
    /// The name of the remove to delete.
    #[clap(long)]
    pub name: String,

    /// When set, the related package will also be removed.
    #[clap(long)]
    pub cascade: bool,
}

#[derive(Parser, Debug)]
#[clap(color = ColorChoice::Always)]
pub enum AddRemoteSubCommand {
    /// Create a remote based on the local filesystem
    Local(AddRemoteLocalSubCommand),
    /// Create a remote backed by an S3 bucket
    S3(AddRemoteS3SubCommand),
}

#[derive(Parser, Debug)]
pub struct AddRemoteLocalSubCommand {
    /// Name for the remove. This name must be unique between remote packages.
    /// Usually this should be the name of the package.
    #[clap(long)]
    pub name: String,

    /// The Location on disk to install the package from.
    #[clap(long)]
    pub path: String,
}

#[derive(Parser, Debug)]
pub struct AddRemoteS3SubCommand {
    /// Name for the remove. This name must be unique between remote packages.
    /// Usually this should be the name of the package.
    #[clap(long)]
    pub name: String,

    /// The URL to download the package from.
    #[clap(long)]
    pub url: String,

    #[clap(long, arg_enum, default_value("anonymous"))]
    pub auth: S3AuthType,

    #[clap(long, required_if_eq("auth", "host"))]
    /// Location of script, that will export environment variables to auth with S3
    pub auth_script: Option<String>,
}

#[derive(ArgEnum, Debug, PartialEq, Clone)]
pub enum S3AuthType {
    Anonymous,
    Host,
}

#[derive(Parser, Debug)]
pub struct ToolAddArgs {}

#[derive(Parser, Debug)]
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

#[derive(Parser, Debug)]
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

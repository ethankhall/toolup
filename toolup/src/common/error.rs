use std::path::PathBuf;
use std::fmt;

#[macro_export]
macro_rules! err {
    ($x:expr) => {
        return Err(CliError::from((crate::common::error::ErrorCallSite::new(line!(), file!()), $x)));
    }
}

macro_rules! from {
    ($e_type:path, $sub_type:ty) => {
        impl std::convert::From<(ErrorCallSite, $sub_type)> for CliError {
            fn from(sub: (ErrorCallSite, $sub_type)) -> CliError {
                $e_type(sub.0, sub.1)
            }
        }
    }
}

#[derive(Debug)]
pub struct ErrorCallSite {
    line: u32,
    file: String
}

impl ErrorCallSite {
    pub fn new(line: u32, file: &str) -> Self {
        ErrorCallSite { line, file: s!(file) }
    }
}

impl fmt::Display for ErrorCallSite {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}#{}", self.file, self.line)
    }
}

#[derive(Debug)]
pub enum CliError {
    Unknown(ErrorCallSite, ()),
    IO(ErrorCallSite, IOError),
    Config(ErrorCallSite, ConfigError),
    API(ErrorCallSite, ApiError)
}

macro_rules! write_with_line {
    ($f:expr, $s:expr, $e:expr) => {
        if log_enabled!(log::Level::Debug) || log_enabled!(log::Level::Trace) {
            write!($f, "{} [{}] {:?}", $s, $e.get_error_code(), $e)
        } else {
            write!($f, "[{}] {:?}", $e.get_error_code(), $e)
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CliError::Unknown(site, _) => write!(f, "[{}] Unknown Error... Sorry!", site),
            CliError::IO(site, e) => write_with_line!(f, site, e),
            CliError::Config(site, e) => write_with_line!(f, site, e),
            CliError::API(site, e) => write_with_line!(f, site, e)
        }
    }
}

impl std::convert::Into<i32> for CliError {
    fn into(self) -> i32 {
        match self {
            CliError::Unknown(_, _) => 9,
            CliError::IO(_, _) => 10,
            CliError::Config(_, _) => 11,
            CliError::API(_, _) => 12
        }
    }
}


from!(CliError::IO, IOError);
from!(CliError::Config, ConfigError);
from!(CliError::API, ApiError);
from!(CliError::Unknown, ());

trait ErrorCode {
    fn get_error_code(&self) -> String;
}

#[derive(Debug, ErrorCode)]
#[toolup(error_prefix = "IO")]
pub enum IOError {
    UnableToReadFile(PathBuf, String),
    GernalIOError(String),
    UnableToMoveArtifact(String),
    UnableToExtractFile(String)
}

#[derive(Debug, ErrorCode)]
#[toolup(error_prefix = "CFG")]
pub enum ConfigError {
    ConfigFormatError(String),
    ConfigFileNotFound(PathBuf),
    ToolNotFound(String),
    UnableToWriteConfig(String)
}

#[derive(Debug, ErrorCode)]
#[toolup(error_prefix = "API")]
pub enum ApiError {
    UnableToContactGitHub(String),
    CallWasNotSuccessful(String),
    GitHubTokenNotProvided,
    UnableToDownloadArtifact(String)
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::IO(ErrorCallSite::new(0, "unknown"), IOError::GernalIOError(e.to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_error_codes() {
        let text = s!(format!("{}", make_err().err().unwrap()));
        assert!(text.ends_with("[CFG-001] ConfigFormatError(\"fooooo\")"));
    }

    fn make_err() -> Result<(), CliError> {
        err!(ConfigError::ConfigFormatError(s!("fooooo")))
    }
}
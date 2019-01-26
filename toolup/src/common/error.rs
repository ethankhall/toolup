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
    Config(ErrorCallSite, ConfigError)
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
            CliError::Config(site, e) => write_with_line!(f, site, e)
        }
    }
}

impl std::convert::Into<i32> for CliError {
    fn into(self) -> i32 {
        match self {
            CliError::Unknown(_, _) => 9,
            CliError::IO(_, _) => 10,
            CliError::Config(_, _) => 11
        }
    }
}


from!(CliError::IO, IOError);
from!(CliError::Config, ConfigError);
from!(CliError::Unknown, ());

trait ErrorCode {
    fn get_error_code(&self) -> String;
}

#[derive(Debug, ErrorCode)]
#[toolup(error_prefix = "IO")]
pub enum IOError {
    UnableToReadFile(PathBuf, String)
}

#[derive(Debug, ErrorCode)]
#[toolup(error_prefix = "CFG")]
pub enum ConfigError {
    ConfigFormatError(String),
    ConfigFileNotFound(PathBuf)
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
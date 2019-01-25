use std::path::PathBuf;
use std::fmt;

#[macro_export]
macro_rules! err {
    ($x:expr) => {
        return Err(CliError::from($x));
    };
}

macro_rules! from {
    ($e_type:path, $sub_type:ty) => {
        impl std::convert::From<$sub_type> for CliError {
            fn from(sub: $sub_type) -> CliError {
                $e_type(sub)
            }
        }
    }
}

#[derive(Debug)]
pub enum CliError {
    IO(IOError),
    Config(ConfigError)
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CliError::IO(e) => write!(f, "{} {:?}", e.get_error_code(), e),
            CliError::Config(e) => write!(f, "{} {:?}", e.get_error_code(), e)
        }
    }
}

from!(CliError::IO, IOError);
from!(CliError::Config, ConfigError);

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
    ConfigFormatError(String)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_error_codes() {
        let text = format!("{}", CliError::Config(ConfigError::ConfigFormatError(s!("fooooo"))));

        assert_eq!("CFG-001 ConfigFormatError(\"fooooo\")", text);
    }
}
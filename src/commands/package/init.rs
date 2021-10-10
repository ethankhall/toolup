use async_trait::async_trait;
use thiserror::Error;
use std::fs::{File};
use std::io::Write;

use crate::model::UserDefinedPackage;
use crate::commands::SubCommandExec;
use crate::cli::*;

#[derive(Error, Debug)]
pub enum InitPackageError {
    #[error(transparent)]
    TomlError(#[from] toml::ser::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[async_trait]
impl SubCommandExec<InitPackageError> for InitToolSubCommand {
    async fn execute(self) -> Result<(), InitPackageError> {
        let udp = UserDefinedPackage { name: "clu", entrypoints: vec!["clu"], version: "1.0.0" };

        let definition = toml::to_string_pretty(&udp)?;

        let mut f = File::create(self.output_file)?;
        f.write_all(definition.as_bytes()).expect("Unable to write data");

        Ok(())
    }
}
use async_trait::async_trait;
use std::fs::File;
use std::io::Write;
use thiserror::Error;

use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::model::UserDefinedPackage;
use crate::util::GlobalFolders;

#[derive(Error, Debug)]
pub enum InitPackageError {
    #[error(transparent)]
    Toml(#[from] toml::ser::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Uknown(#[from] anyhow::Error),
}

#[async_trait]
impl SubCommandExec<InitPackageError> for InitToolSubCommand {
    async fn execute(self, _global_folder: &GlobalFolders) -> Result<(), InitPackageError> {
        let udp = UserDefinedPackage {
            name: "clu",
            entrypoints: vec!["clu"],
            version: "1.0.0",
        };

        let definition = toml::to_string_pretty(&udp)?;

        let mut f = File::create(self.output_file)?;
        f.write_all(definition.as_bytes())
            .expect("Unable to write data");

        Ok(())
    }
}

use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::model::*;
use crate::util::GlobalFolders;
use async_trait::async_trait;
use chrono::Duration;
use std::fs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AddRemoteError {
    #[error(transparent)]
    StateError(#[from] crate::state::StateError),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[async_trait]
impl SubCommandExec<AddRemoteError> for AddRemoteSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), AddRemoteError> {
        let auth_strategy = match self.auth_script {
            Some(path) => AuthStrategy::Script(AuthScript { script_path: path }),
            None => AuthStrategy::None,
        };
        let s3_package = S3PackageRepository {
            url: self.url.clone(),
            auth_strategy,
        };
        let remote_package = RemotePackage {
            name: self.name.clone(),
            update_period_seconds: Duration::days(1).num_seconds(),
            repository: PackageRepository::S3(s3_package),
        };

        let pretty_json = serde_json::to_string_pretty(&remote_package)?;
        let config_file = global_folder.make_remote_tool_config(&self.name);
        let parent = config_file
            .parent()
            .expect("Should be able to find config dir.");
        fs::create_dir_all(parent)?;

        fs::write(config_file, pretty_json)?;
        Ok(())
    }
}

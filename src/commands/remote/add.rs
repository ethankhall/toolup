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
    State(#[from] crate::state::StateError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Uknown(#[from] anyhow::Error),
}

#[async_trait]
impl SubCommandExec<AddRemoteError> for AddRemoteSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), AddRemoteError> {
        match self {
            AddRemoteSubCommand::Local(local) => local.execute(global_folder).await,
            AddRemoteSubCommand::S3(s3) => s3.execute(global_folder).await,
        }
    }
}

#[async_trait]
impl SubCommandExec<AddRemoteError> for AddRemoteLocalSubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), AddRemoteError> {
        let local_package = LocalPackageRepository {
            path: self.path.clone(),
        };
        let remote_package = RemotePackage {
            name: self.name.clone(),
            update_period_seconds: Duration::days(1).num_seconds(),
            repository: PackageRepository::Local(local_package),
        };

        add_remote_package(&self.name, remote_package, global_folder)
    }
}

#[async_trait]
impl SubCommandExec<AddRemoteError> for AddRemoteS3SubCommand {
    async fn execute(self, global_folder: &GlobalFolders) -> Result<(), AddRemoteError> {

        let auth_strategy = match self.auth {
            S3AuthType::Anonymous => AuthStrategy::None,
            S3AuthType::Host => {
                match self.auth_script {
                    Some(path) => AuthStrategy::Script(AuthScript { script_path: path }),
                    None => AuthStrategy::DefaultAwsAuth,
                }
            }
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
        add_remote_package(&self.name, remote_package, global_folder)
    }
}

fn add_remote_package(
    name: &str,
    package: RemotePackage,
    global_folder: &GlobalFolders,
) -> Result<(), AddRemoteError> {
    let pretty_json = serde_json::to_string_pretty(&package)?;
    let config_file = global_folder.make_remote_tool_config(&name);
    let parent = config_file
        .parent()
        .expect("Should be able to find config dir.");
    fs::create_dir_all(parent)?;

    fs::write(config_file, pretty_json)?;
    Ok(())
}

use clap::Clap;
use async_trait::async_trait;
use thiserror::Error;
use std::fs::{read_to_string, File};
use std::io::prelude::*;
use std::collections::BTreeMap;
use walkdir::{WalkDir};
use std::path::*;
use path_absolutize::*;
use flate2::{Compression, write::GzEncoder};
use tracing::{info, debug, instrument};

use crate::model::{GENERATED_FILE_NAME, UserDefinedPackage, GeneratedDefinedPackage};
use crate::commands::SubCommandExec;

#[derive(Error, Debug)]
pub enum ArchivePackageError {
    #[error("Unable to find `{target}`.")]
    TargetDoesNotExist {
        target: String
    },
    #[error("Unable to process {dir} due to {err}.")]
    UnableToWalkDir {
        dir: String,
        err: walkdir::Error
    },
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error(transparent)]
    JsonSeError(#[from] serde_json::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[derive(Clap, Debug)]
pub struct ArchiveToolSubCommand {
    /// Location on disk that has the artifact directory ready.
    /// 
    /// All files relative to this directory will be packaged up for distribution.
    /// There is a limit of 128 MiB total uncompressed files.
    #[clap(short, long)]
    target_dir: String,

    /// The config file that describes the tool that is being packaged.
    #[clap(long = "config")]
    application_config: String,

    /// Location to write the package to.
    #[clap(long)]
    archive_path: String,
}

#[async_trait]
impl SubCommandExec<ArchivePackageError> for ArchiveToolSubCommand {
    async fn execute(self) -> Result<(), ArchivePackageError> {
        let definition = read_to_string(&self.application_config)?;
        let definition: UserDefinedPackage = toml::from_str(&definition)?;
        let mut files_to_package: BTreeMap<String, String> = BTreeMap::default();

        let target_dir = Path::new(&self.target_dir);
        if !target_dir.exists() {
            return Err(ArchivePackageError::TargetDoesNotExist { target: self.target_dir });
        }

        let target_dir = std::fs::canonicalize(target_dir)?;
        let target_dir_absolute_path = target_dir.display().to_string();

        let mut entrypoint_paths = Vec::new();

        for entrypoint in &definition.entrypoints {
            entrypoint_paths.push(validate_entrypoint(entrypoint, &target_dir)?);
        }

        for entry in WalkDir::new(&target_dir) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    return Err(ArchivePackageError::UnableToWalkDir { dir: target_dir_absolute_path, err: e});
                }
            };

            if entry.file_type().is_dir() {
                continue;
            }

            let archive_path = entry.path().strip_prefix(&target_dir).expect("Base path to be well known");
            files_to_package.insert(archive_path.display().to_string(), entry.path().display().to_string());
        }

        debug!("Files to include in archive are: {:?}", files_to_package);

        let archive = create_archive(entrypoint_paths, &definition, files_to_package).await?;
        let mut e = GzEncoder::new(File::create(&self.archive_path)?, Compression::default());
        e.write_all(&archive)?;
        e.finish()?;

        info!(target: "user", "Finished creating artifact at path {}", self.archive_path);

        Ok(())
    }
}

fn validate_entrypoint(entrypoint: &str, archive_root: &Path) -> Result<String, ArchivePackageError> {
    let entrypoint_path = Path::join(&archive_root, entrypoint);
    if !entrypoint_path.exists() {
        return Err(ArchivePackageError::TargetDoesNotExist { target: entrypoint_path.display().to_string() });
    }
    let entrypoint_path = entrypoint_path.absolutize().unwrap();
    let entrypoint_path = entrypoint_path.strip_prefix(&archive_root).expect("Base path to be well known").display().to_string();

    Ok(entrypoint_path)
}

#[instrument(skip(entrypoint_paths, package, artifacts))]
async fn create_archive(entrypoint_paths: Vec<String>, package: &UserDefinedPackage<'_>, artifacts: BTreeMap<String, String>) -> Result<Vec<u8>, ArchivePackageError> {
    use tar::{Builder, Header};
    use std::convert::TryInto;

    let mut archive = Builder::new(Vec::new());
    let mut definition = GeneratedDefinedPackage {
        name: package.name.to_string(),
        entrypoints: entrypoint_paths,
        version: package.version.to_string(),
        file_hashes: Default::default(), 
        achived_at: chrono::Utc::now()
    };

    for (archive_name, file_path) in artifacts.into_iter() {
        info!("Archiving {}", archive_name);
        let mut buffer = Vec::new();
        let mut file = File::open(file_path)?;
        let size = file.read_to_end(&mut buffer)?;

        let hex = crate::util::get_hash_for_contents(buffer.clone());
        definition.file_hashes.insert(archive_name.clone(), hex);

        let mut header = Header::new_gnu();
        header.set_size(size.try_into().unwrap());
        header.set_metadata(&file.metadata()?);
        header.set_gid(1000);
        header.set_uid(1000);
        header.set_cksum();

        archive.append_data(&mut header, archive_name, &mut buffer.as_slice())?;
    }

    let definition_stream = serde_json::to_string_pretty(&definition)?;
    let definition_stream = definition_stream.as_bytes();

    let mut header = Header::new_gnu();
    header.set_size(definition_stream.len().try_into().unwrap());
    header.set_mode(0o644);
    header.set_gid(1000);
    header.set_uid(1000);
    header.set_cksum();
    header.set_mtime(definition.achived_at.timestamp().try_into().unwrap());
    archive.append_data(&mut header, GENERATED_FILE_NAME, definition_stream)?;

    Ok(archive.into_inner()?)
}
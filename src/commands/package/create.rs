use async_trait::async_trait;
use flate2::{write::GzEncoder, Compression};
use path_absolutize::*;
use std::collections::BTreeMap;
use std::fs::{read_to_string, File};
use std::io::prelude::*;
use std::path::*;
use thiserror::Error;
use tracing::{debug, info, instrument};
use walkdir::WalkDir;

use crate::cli::*;
use crate::commands::SubCommandExec;
use crate::model::{GeneratedDefinedPackage, UserDefinedPackage, GENERATED_FILE_NAME};
use crate::util::GlobalFolders;

#[derive(Error, Debug)]
pub enum ArchivePackageError {
    #[error("Unable to find `{target}`.")]
    TargetDoesNotExist { target: String },
    #[error("`{target}` is not a file.")]
    TargetIsNotFile { target: String },
    #[error("Unable to process {dir} due to {err}.")]
    UnableToWalkDir { dir: String, err: walkdir::Error },
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error(transparent)]
    JsonSeError(#[from] serde_json::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[async_trait]
impl SubCommandExec<ArchivePackageError> for ArchiveToolSubCommand {
    async fn execute(self, _global_folder: &GlobalFolders) -> Result<(), ArchivePackageError> {
        debug!("Reading definition from {}", self.application_config);

        let application_config_path = Path::new(&self.application_config);
        if !application_config_path.exists() {
            return Err(ArchivePackageError::TargetDoesNotExist {
                target: self.target_dir,
            });
        }

        if !application_config_path.is_file() {
            return Err(ArchivePackageError::TargetIsNotFile {
                target: self.target_dir,
            });
        }

        let definition = read_to_string(&application_config_path)?;
        let definition: UserDefinedPackage = toml::from_str(&definition)?;
        let mut files_to_package: BTreeMap<String, String> = BTreeMap::default();

        let target_dir = Path::new(&self.target_dir);
        if !target_dir.exists() {
            return Err(ArchivePackageError::TargetDoesNotExist {
                target: self.target_dir,
            });
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
                    return Err(ArchivePackageError::UnableToWalkDir {
                        dir: target_dir_absolute_path,
                        err: e,
                    });
                }
            };

            if entry.file_type().is_dir() {
                continue;
            }

            let entry_path = entry
                .path()
                .strip_prefix(&target_dir)
                .expect("Base path to be well known");
            files_to_package.insert(
                entry_path.display().to_string(),
                entry.path().display().to_string(),
            );
        }

        debug!("Files to include in archive are: {:?}", files_to_package);

        let archive_path = Path::new(&self.archive_dir).join(format!(
            "{name}-{version}.tar.gz",
            name = definition.name.replace(' ', "_"),
            version = definition.version
        ));

        let archive = create_archive(entrypoint_paths, &definition, files_to_package).await?;

        debug!("Compressing files");
        let mut e = GzEncoder::new(File::create(&archive_path)?, Compression::default());
        e.write_all(&archive)?;
        e.finish()?;

        info!(target: "user", "Finished creating artifact at path {}", archive_path.display().to_string());

        Ok(())
    }
}

fn validate_entrypoint(
    entrypoint: &str,
    archive_root: &Path,
) -> Result<String, ArchivePackageError> {
    let entrypoint_path = Path::join(archive_root, entrypoint);
    if !entrypoint_path.exists() {
        return Err(ArchivePackageError::TargetDoesNotExist {
            target: entrypoint_path.display().to_string(),
        });
    }
    let entrypoint_path = entrypoint_path.absolutize().unwrap();
    let entrypoint_path = entrypoint_path
        .strip_prefix(&archive_root)
        .expect("Base path to be well known")
        .display()
        .to_string();

    Ok(entrypoint_path)
}

#[instrument(skip(entrypoint_paths, package, artifacts))]
async fn create_archive(
    entrypoint_paths: Vec<String>,
    package: &UserDefinedPackage<'_>,
    artifacts: BTreeMap<String, String>,
) -> Result<Vec<u8>, ArchivePackageError> {
    use std::convert::TryInto;
    use tar::{Builder, Header};

    let mut entrypoint_map = BTreeMap::new();
    for entrypoint in entrypoint_paths {
        let command_name = Path::new(&entrypoint)
            .file_name()
            .expect("The state file to have a valid filename")
            .to_os_string()
            .into_string()
            .expect("State file to have a valid filename.");
        entrypoint_map.insert(command_name, entrypoint);
    }

    let mut archive = Builder::new(Vec::new());
    let mut definition = GeneratedDefinedPackage {
        name: package.name.to_string(),
        entrypoints: entrypoint_map,
        version: package.version.to_string(),
        file_hashes: Default::default(),
        achived_at: chrono::Utc::now(),
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

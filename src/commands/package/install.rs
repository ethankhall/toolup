use clap::Clap;
use async_trait::async_trait;
use thiserror::Error;
use std::fs::{File, self};
use std::io::prelude::*;
use std::path::*;
use flate2::{read::GzDecoder};
use tar::{Archive};
use tracing::{info, debug, instrument};

use crate::model::{GENERATED_FILE_NAME, GeneratedDefinedPackage};
use crate::commands::SubCommandExec;
use crate::util::{set_executable, get_hash_for_contents};

#[derive(Error, Debug)]
pub enum InstallPackageError {
    #[error("Unable to extract `{path}`. OS Error: {error}")]
    UnableToExtractPackage {
        path: String,
        error: std::io::Error,
    },
    #[error("Unable to open and read `{path}`. OS Error: {error}")]
    UnableToReadPackage {
        path: String,
        error: std::io::Error,
    },
    #[error("Package was currupted! Found {filename} which was expected to have a checksum {expected} but instead was {computed}.")]
    CurruptedArchive { filename: String, expected: String, computed: String},
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[derive(Clap, Debug)]
pub struct InstallToolSubCommand {
    /// Location on disk has the pre-built package.
    /// 
    /// This package will be extracted, and placed inside your user directory.
    /// When a package is isntalled locally, it will no longer support refreshs
    /// from an upstream source.
    #[clap(short, long)]
    archive_path: String,

    /// If the package already exists, overwrite it.
    /// 
    /// When set, toolup will clearn out the destination directory if it exists.
    #[clap(short, long)]
    overwrite: bool,

    /// Override the location to install the package.
    /// 
    /// This option will allow you to install the package in a custom directory,
    /// instead of the default one managed by toolup.
    #[clap(short, long)]
    tool_root_dir: Option<String>,
}

#[async_trait]
impl SubCommandExec<InstallPackageError> for InstallToolSubCommand {
    async fn execute(self) -> Result<(), InstallPackageError> {

        let tool_root_dir = match self.tool_root_dir {
            Some(path) => path,
            None => crate::util::LATEST_INSTALL_DIR.to_string()
        };

        let tool_root_dir = Path::new(&tool_root_dir);
        let archive_path = Path::new(&self.archive_path); 
        let tmp_extract_dir = tool_root_dir.join(format!("tmp.{}", chrono::Utc::now().timestamp()));
        let package_def = extract_and_validate(archive_path, &tmp_extract_dir).await?;
        
        move_package_to_correct_location(&tmp_extract_dir, tool_root_dir, package_def, self.overwrite).await?;
        Ok(())
    }
}

#[instrument(skip(temp_dir))]
async fn extract_and_validate(package_file: &Path, temp_dir: &Path) -> Result<GeneratedDefinedPackage, InstallPackageError> {
    let file = match fs::File::open(&package_file) {
        Ok(file) => file,
        Err(e) => return Err(InstallPackageError::UnableToReadPackage { path: package_file.display().to_string(), error: e }),
    };

    let mut gz: Vec<u8> = Vec::new();

    let mut d = GzDecoder::new(file);
    if let Err(e) = d.read_to_end(&mut gz) {
        return Err(InstallPackageError::UnableToExtractPackage { path: package_file.display().to_string(), error: e });
    }

    debug!("Temp dir to extract archive to {}", temp_dir.display().to_string());

    let mut a = Archive::new(gz.as_slice());
    if let Err(e) = a.unpack(&temp_dir) {
        return Err(InstallPackageError::UnableToExtractPackage { path: package_file.display().to_string(), error: e });
    }

    let package_def_file = temp_dir.join(GENERATED_FILE_NAME);
    let archive_def: GeneratedDefinedPackage = serde_json::from_reader(File::open(package_def_file)?)?;

    for (filename, hash) in &archive_def.file_hashes {
        valdiate_file(temp_dir.join(filename), &hash).await?;
    }

    for entrypoint in &archive_def.entrypoints {
        set_executable(&temp_dir.join(entrypoint));
    }

    Ok(archive_def)
}

async fn move_package_to_correct_location(temp_dir: &Path, tool_root: &Path, package: GeneratedDefinedPackage, overwrite: bool) -> Result<(), InstallPackageError> {
    let unix_friendly_name = package.name.replace(' ', "_");
    let real_dest = tool_root.to_owned().join(&unix_friendly_name).join(package.version);
   
    if real_dest.exists() && overwrite {
        info!(target: "user", "Cleading up old install of {}", package.name);
        fs::remove_dir_all(&real_dest)?;
    }

    info!(target: "user", "Installing {} at {}", package.name, real_dest.display().to_string());

    fs::create_dir_all(real_dest.parent().expect("Partent to exist"))?;
    fs::rename(temp_dir, real_dest)?;

    Ok(())
}

#[instrument]
async fn valdiate_file(path: PathBuf, expected_checksum: &str) -> Result<(), InstallPackageError> {
    debug!("Validating file");
    let mut file = File::open(&path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let computed_hash = get_hash_for_contents(buffer.clone());
    if expected_checksum != computed_hash {
        return Err(InstallPackageError::CurruptedArchive { filename: path.display().to_string(), expected: expected_checksum.to_string(), computed: computed_hash})
    }
    Ok(())
}
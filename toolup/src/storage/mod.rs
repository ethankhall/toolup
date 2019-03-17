pub mod github;
pub mod link;

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use flate2::read::GzDecoder;
use tar::Archive;

use crate::common::error::*;
use crate::common::progress::*;
use crate::config::lock::*;
use crate::config::ConfigContainer;
use crate::err;

pub fn download_tools(config: &ToolLock, versions: &Vec<ToolVersion>) -> Result<(), CliError> {
    let pb = ProgressBarHelper::new(versions.len() as u64, ProgressBarType::Downloading);

    for version in versions {
        debug!("Attempting to download {:?}", version);
        pb.inc(&version.name);
        match download_tool(&version, &config.get_tokens()) {
            Ok(true) => {}
            Ok(false) => {
                warn!("Unable to download {}", version.name);
            }
            Err(e) => return Err(e),
        }
    }

    pb.done();
    Ok(())
}

pub fn download_tool(tool: &ToolVersion, tokens: &Tokens) -> Result<bool, CliError> {
    if tool.artifact_exists() {
        return Ok(true);
    }

    debug!("Downloading tool: {:?}", tool);

    let url = match &tool.download_url {
        Some(url) => url.to_string(),
        None => {
            eprintln!("Unable to download {}", &tool.name);
            err!(ConfigError::ToolCanNotBeDownloaded(s!(tool.name)));
        }
    };

    let client = reqwest::Client::new();
    let req = client.get(&url);

    let req = match &tool.auth_token_source {
        AuthTokenSource::None => req,
        AuthTokenSource::GitHub => {
            let token = github::get_github_token(&tokens)?;
            req.header(reqwest::header::AUTHORIZATION, token)
        }
    };

    let mut download_dir = tool.get_download_dir();
    fs::create_dir_all(&download_dir)?;

    let part_path = download_dir.join(format!("../{}.part", tool.version));

    match req.send() {
        Err(e) => err!(ApiError::UnableToDownloadArtifact(e.to_string())),
        Ok(mut response) => {
            if response.status().is_success() {
                let mut file = fs::File::create(part_path.clone())?;
                if let Err(e) = response.copy_to(&mut file) {
                    err!(ApiError::CallWasNotSuccessful(e.to_string()))
                }
            } else {
                match response.text() {
                    Ok(text) => err!(ApiError::CallWasNotSuccessful(text)),
                    Err(e) => err!(ApiError::CallWasNotSuccessful(e.to_string())),
                }
            }
        }
    }

    download_dir.pop();
    download_dir.push(&tool.version);

    debug!(
        "Downloading {} into {:#?} using {:#?}",
        tool.name, download_dir, part_path
    );

    fs::create_dir_all(&download_dir)?;

    match tool.art_type {
        ArtifactType::Raw => {
            if let Err(e) = fs::rename(part_path, download_dir.join(&tool.exec_path)) {
                err!(IOError::UnableToMoveArtifact(e.to_string()))
            }
        }
        ArtifactType::Tgz => {
            let file = match fs::File::open(&part_path) {
                Ok(file) => file,
                Err(e) => err!(IOError::UnableToReadFile(part_path, e.to_string())),
            };

            let mut gz: Vec<u8> = Vec::new();

            let mut d = GzDecoder::new(file);
            if let Err(e) = d.read_to_end(&mut gz) {
                err!(IOError::UnableToExtractFile(e.to_string()))
            }

            let mut a = Archive::new(gz.as_slice());
            if let Err(e) = a.unpack(download_dir) {
                err!(IOError::UnableToExtractFile(e.to_string()))
            }

            let _ = fs::remove_file(&part_path);
        }
        ArtifactType::Zip => {
            let file = match fs::File::open(&part_path) {
                Ok(file) => file,
                Err(e) => err!(IOError::UnableToReadFile(part_path, e.to_string())),
            };

            let mut archive = zip::ZipArchive::new(file).unwrap();

            for i in 0..archive.len() {
                let mut path = download_dir.clone();
                let mut file = archive.by_index(i).unwrap();
                path.push(file.sanitized_name());

                let mut contents: Vec<u8> = Vec::new();
                if let Err(e) = file.read_to_end(&mut contents) {
                    err!(IOError::UnableToExtractFile(e.to_string()))
                }

                if let Err(e) = fs::write(path, contents.as_slice()) {
                    err!(IOError::UnableToExtractFile(e.to_string()))
                }
            }

            let _ = fs::remove_file(&part_path);
        }
    }

    set_executable(&tool.exec_path());

    Ok(true)
}

#[cfg(target_family = "unix")]
fn set_executable(path: &PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o774);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(target_family = "windows")]
fn set_executable(_path: &PathBuf) {}

pub fn pull_for_latest() -> Result<(), CliError> {
    let mut lock = ToolLock::get_global_lock().clone();

    let global_config_tools = lock.get_definations();
    let pb = ProgressBarHelper::new(global_config_tools.len() as u64, ProgressBarType::Updating);

    for tool in global_config_tools {
        let name = tool.name;
        let config = tool.config;
        pb.inc(&name);

        let versions = match config.version_source() {
            VersionSource::GitHub { owner, repo } => {
                let token = github::get_github_token(&lock.get_tokens())?;
                github::get_current_details(s!(owner), s!(repo), token, &name, &config.artifact)?
            }
        };

        lock.add_all(versions);
    }

    pb.done();

    ConfigContainer::set_lock_config(lock.clone());
    ConfigContainer::write_config()?;

    Ok(())
}

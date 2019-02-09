pub mod github;
pub mod lock;

use std::path::{Path, PathBuf};
use std::io::Read;
use std::fs;

use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use tar::Archive;

use self::lock::*;
use crate::common::error::*;
use crate::common::model::*;
use crate::{ConfigFiles, err};

pub fn download_tools(config: &ToolLock, versions: Vec<ToolVersion>) -> Result<(), CliError> {
    let pb = ProgressBar::new(versions.len() as u64);
    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.dim} {spinner} [{pos}/{len}] Downloading {wide_msg}");
    pb.set_style(spinner_style.clone());
    pb.enable_steady_tick(100);

    for version in versions {
        debug!("Attempting to download {:?}", version);

        pb.inc(1);
        pb.set_message(&version.name);
         match download_tool(&version, &config.get_tokens()) {
            Ok(true) => {},
            Ok(false) => {
                warn!("Unable to download {}", version.name);
            },
            Err(e) => return Err(e)
        }
    }

    Ok(())
}

pub fn download_tool(tool: &ToolVersion, tokens: &Tokens) -> Result<bool, CliError> {
    if tool.artifact_exists() {
        return Ok(true);
    }

    let client = reqwest::Client::new();
    let req = client.get(&tool.download_url);

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
                    Err(e) => err!(ApiError::CallWasNotSuccessful(e.to_string()))
                }
            }
        },
    }

    download_dir.pop();
    download_dir.push(&tool.version);

    debug!("Downloading {} into {:#?} using {:#?}", tool.name, download_dir, part_path);

    fs::create_dir_all(&download_dir)?;

    match tool.art_type {
        ArtifactType::Raw => {
            if let Err(e) = fs::rename(part_path, download_dir.join(&tool.exec_path)) {
                err!(IOError::UnableToMoveArtifact(e.to_string()))
            }
        },
        ArtifactType::Tgz => {
            let file = match fs::File::open(&part_path) {
                Ok(file) => file,
                Err(e) => err!(IOError::UnableToReadFile(part_path, e.to_string()))
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
        },
        ArtifactType::Zip => {
            let file = match fs::File::open(&part_path) {
                Ok(file) => file,
                Err(e) => err!(IOError::UnableToReadFile(part_path, e.to_string()))
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

    Ok(true)
}

pub fn update_global_state(lock: ToolLock, config: &GlobalConfig) -> Result<ToolLock, CliError> {
    lock.update_tokens(&config.tokens);

    let global_config_tools = config.tools();
    let pb = ProgressBar::new(global_config_tools.len() as u64);
    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.dim} {spinner} [{pos}/{len}] Updating {wide_msg}");
    pb.set_style(spinner_style.clone());
    pb.enable_steady_tick(100);

    for (name, tool) in global_config_tools.into_iter() {
        pb.inc(1);
        pb.set_message(&name);

        let versions = match tool.version_source() {
            VersionSource::GitHub { owner, repo } => {
                let token = github::get_github_token(&lock.get_tokens())?;
                github::get_current_details(s!(owner), s!(repo), token, &name, &tool.artifact)?
            }
        };

        lock.add_all(versions);
    }

    pb.finish_and_clear();
    
    match write_lock(&lock) {
        Ok(_) => Ok(lock),
        Err(e) => Err(e)
    }
}
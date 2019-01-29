pub mod model;
pub mod github;

use std::path::{Path};
use std::io::Read;
use std::fs;

use flate2::read::GzDecoder;
use directories::ProjectDirs;
use indicatif::{ProgressBar, ProgressStyle};
use tar::Archive;

use crate::common::error::*;
use crate::common::model::*;
use self::model::*;
use crate::err;

pub fn download_tool(state: GlobalState, tool_name: String) -> Result<bool, CliError> {
    let tool = match state.get_tool(&tool_name) {
        Some(tool) => tool,
        None => return Ok(false)
    };

    let version = match tool.get_version_to_download() {
        Some(version) => version,
        None => return Ok(false)
    };

    let client = reqwest::Client::new();
    let mut req = client.get(&version.download_url);

    let mut req = match &tool.auth {
        AuthConfig::None => req,
        AuthConfig::Authorization(value) => req.header(reqwest::header::AUTHORIZATION, value.clone())
    };

    let project_dirs = ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
    let mut download_dir = project_dirs.cache_dir().to_path_buf();
    download_dir.push("download");
    download_dir.push(tool_name);
    fs::create_dir_all(&download_dir)?;

    download_dir.push(format!("{}.part", version.name));

    match req.send() {
        Err(e) => err!(ApiError::UnableToDownloadArtifact(e.to_string())),
        Ok(mut response) => { 
            if response.status().is_success() {
                let mut file = fs::File::create(download_dir.clone())?;
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

    let temp_file = download_dir.to_path_buf();
    download_dir.pop();
    download_dir.push(version.name);

    fs::create_dir_all(&download_dir)?;

    match version.container {
        ArtifactType::Raw => {
            if let Err(e) = fs::rename(temp_file, download_dir) {
                err!(IOError::UnableToMoveArtifact(e.to_string()))
            }
        },
        ArtifactType::TGZ => {
            let file = match fs::File::open(&temp_file) {
                Ok(file) => file,
                Err(e) => err!(IOError::UnableToReadFile(temp_file, e.to_string()))
            };

            let mut gz: Vec<u8> = Vec::new();

            let mut d = GzDecoder::new(file);
            if let Err(e) = d.read_to_end(&mut gz) {
                err!(IOError::UnableToExtractFile(e.to_string()))
            }
            
            let mut a = Archive::new(gz.as_slice());
            if let Err(e) =  a.unpack(download_dir) {
                err!(IOError::UnableToExtractFile(e.to_string()))
            }
        },
        ArtifactType::Zip => {}
    }

    Ok(false)
}

pub fn get_global_state(config: &GlobalConfig) -> Result<GlobalState, CliError> {
    let mut lock = match read_existing_lock() {
        Some(lock) => lock,
        None => GlobalState::default()
    };

    let global_config_tools = config.tools();
    let pb = ProgressBar::new(global_config_tools.len() as u64);
    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.dim} {spinner} [{pos}/{len}] Updating {wide_msg}");
    pb.set_style(spinner_style.clone());

    for (name, tool) in global_config_tools.into_iter() {
        pb.inc(1);
        pb.set_message(&name);

        let versions = match tool.version_source() {
            VersionSource::GitHub { owner, repo } => 
                github::get_current_details(s!(owner), s!(repo), &config.tokens, tool.artifact.get_name())
        };

        if let Ok(versions) = versions {
            merge_in(&mut lock, name, tool, versions);
        } else {
            warn!("Unable to update {}", name);
        }
    }

    pb.finish_and_clear();

    Ok(lock)
}

fn merge_in(global: &mut GlobalState, name: &String, app: &ApplicationConfig, versions: Vec<VersionUrlResponse>) {
    if !global.tools.contains_key(name) {
        global.tools.insert(name.to_string(), ToolGlobalState::new(name.to_string()));
    }

    let tool: &ToolGlobalState = global.get_tool(&name)
        .expect("Tool to exist, as we've created it if it's missing");

    for version in versions {
        if !tool.has_version(&version.name()) {
            let project_dirs = ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
            let mut download_dir = project_dirs.cache_dir().to_path_buf();
            download_dir.push("download");
            download_dir.push(name);
            download_dir.push(version.name());
            download_dir.push(app.artifact.path_to_art());

            let art_type = match app.artifact {
                ArtifactSource::Zip { name: _, path: _ } => ArtifactType::Zip,
                ArtifactSource::TGZ { name: _, path: _ } => ArtifactType::TGZ,
                ArtifactSource::Raw { name: _ }=> ArtifactType::Raw
            };

            let tool_install = TookInstallDetails { exec_path: s!(download_dir.to_str().unwrap()), downloaded_at: None };
            tool.insert_version(version.name(), ToolVersion::new(version, art_type, tool_install));
        }
    }
}

fn read_existing_lock() -> Option<GlobalState> {
    let project_dirs = ProjectDirs::from("io", "ehdev", "toolup").expect("To create project dirs");
    let config_dir = project_dirs.config_dir();
    let global_path = config_dir.join(Path::new("toolup.lock"));

    if global_path.exists() {
        let contents: String = match fs::read_to_string(&global_path) {
            Ok(contents) => contents,
            Err(err) => return None
        };

        debug!("Contents for global config {:?}", contents);

        return match toml::from_str::<GlobalState>(&contents) {
            Ok(config) => Some(config),
            Err(err) => {
                warn!("Unable to deserialize existing state file, dropping it.");
                None
            }
        };
    }

    None
}
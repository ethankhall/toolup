use std::path::{Path, PathBuf};
use std::cell::RefCell;
use std::fs;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use chrono::serde::ts_milliseconds;

use crate::err;
use crate::common::error::*;
use crate::common::model::Tokens;

pub const NO_DOWNLOAD_URL: &'static str = "No URL";

#[derive(Deserialize, Serialize, Debug)]
pub struct ToolLock {
    tokens: RefCell<Tokens>,
    tools: RwLock<Vec<ToolVersion>>,
    wanted: RwLock<HashMap<String, WantedVersion>>
}

#[derive(Deserialize, Serialize, Debug)]
pub enum WantedVersion {
    Latest,
    Specific(String)
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub struct ToolVersion {
    pub name: String,
    pub version: String,
    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    pub download_url: String,
    pub exec_path: String,
    pub art_type: ArtifactType,
    pub auth_token_source: AuthTokenSource
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub enum AuthTokenSource {
    None,
    GitHub
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ArtifactType {
    Tgz,
    Raw,
    Zip
}

impl ToolLock {
    pub fn add_new(&self, tool_version: ToolVersion) {
        if self.find_version(&tool_version.name, &tool_version.version).is_some() {
            return;
        }

        let mut tools = self.tools.write().unwrap();
        tools.push(tool_version.clone());

        let mut wanted = self.wanted.write().unwrap();
        wanted.entry(tool_version.name).or_insert(WantedVersion::Latest);
    }

    pub fn add_all(&self, tool_versions: Vec<ToolVersion>) {
        for tool in tool_versions {
            self.add_new(tool);
        }
    }

    pub fn find_tool(&self, tool_name: &str) -> Vec<ToolVersion> {
        let tools = self.tools.read().unwrap();
        let result: Vec<ToolVersion> = tools.iter().filter(|x| x.name == tool_name).map(|x| x.clone()).collect();
        result
    }

    pub fn find_version(&self, tool_name: &str, version: &str) -> Option<ToolVersion> {
        let tools = self.tools.read().unwrap();
        tools.iter().find(|x| x.name == tool_name && x.version == version).map(|x| x.clone())
    }
    
    pub fn get_all_tools(&self) -> Vec<ToolVersion> {
        let tools = self.tools.read().unwrap();
        tools.clone()
    }

    pub fn update_tokens(&self, tokens: &Tokens) {
        self.tokens.replace(tokens.clone());
    }

    pub fn get_tokens(&self) -> Tokens {
        self.tokens.borrow().clone()
    }

    pub fn get_all_wanted(&self) -> Vec<ToolVersion> {
        let wanted = self.wanted.read().unwrap();
        wanted.keys().flat_map(|name| self.get_wanted(&name)).collect()
    }

    pub fn get_wanted(&self, tool_name: &str) -> Option<ToolVersion> {
        let wanted = self.wanted.read().unwrap();
        let wanted = match wanted.get(tool_name) {
            Some(value) => value,
            None => &WantedVersion::Latest
        };

        match wanted {
            WantedVersion::Latest => {
                let mut versions = self.find_tool(tool_name);
                versions.sort_by_key(|x| x.created_at);
                match versions.last() {
                    Some(x) => Some(x.clone()),
                    None => None
                }
            }
            WantedVersion::Specific(version) => self.find_version(tool_name, &version)
        }
    }
}

impl std::default::Default for ToolLock {
    fn default() -> Self {
        ToolLock { tokens: RefCell::default(), tools: RwLock::new(Vec::new()), wanted: RwLock::new(HashMap::new()) }
    }
}

impl ToolVersion {
    pub fn get_download_dir(&self) -> PathBuf {
        let mut path = PathBuf::from(crate::CACHE_DIR.as_str());
        path.push("download");
        path.push(&self.name);
        path.push(&self.version);

        path
    }

    pub fn exec_path(&self) -> PathBuf {
        self.get_download_dir().join(&self.exec_path)
    }

    pub fn artifact_exists(&self) -> bool {
        self.exec_path().exists()
    }

    pub fn is_downloadable(&self) -> bool {
        self.download_url != NO_DOWNLOAD_URL
    }
}

impl Ord for ToolVersion {
    fn cmp(&self, other: &ToolVersion) -> Ordering {
        self.created_at.cmp(&other.created_at)
    }
}

impl PartialOrd for ToolVersion {
    fn partial_cmp(&self, other: &ToolVersion) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn write_lock(lock: &ToolLock) -> Result<(), CliError> {
    let lock_path = PathBuf::from(crate::CONFIG_DIR.as_str());
    let global_path = lock_path.join(Path::new("Toolup.lock"));

    if !lock_path.exists() {
        fs::create_dir_all(lock_path)?;
    }

    debug!("Writing lock to {:#?}.", &global_path);

    let text = match toml::to_string(&lock) {
        Ok(text) => text,
        Err(err) => {
            warn!("Unable to seralize state file.");
            err!(ConfigError::UnableToWriteConfig(err.to_string()))
        }
    };

    match fs::write(global_path, text) {
        Ok(_) => Ok(()),
        Err(e) => {
            warn!("Unable to write state file.");
            err!(ConfigError::UnableToWriteConfig(e.to_string()))
        }
    }
}

pub fn read_existing_lock(global_path: PathBuf) -> Option<ToolLock> {
    trace!("Reading config from {:#?}", global_path);

    if global_path.exists() {
        let contents: String = match fs::read_to_string(&global_path) {
            Ok(contents) => contents,
            Err(_) => return None
        };

        return match toml::from_str::<ToolLock>(&contents) {
            Ok(config) => Some(config),
            Err(_) => {
                warn!("Unable to deserialize existing state file, dropping it.");
                None
            }
        };
    }

    None
}

#[cfg(test)]
mod test {

    use super::*;
    use toml;

    #[test]
    fn test_new_and_add() {
        let lock = ToolLock::default();

        let now: DateTime<Utc> = Utc::now();

        let tool_version = ToolVersion { 
            name: s!("foo"),
            version: s!("bar"),
            created_at: now,
            download_url: s!("http://localhost/help"),
            exec_path: s!("foo.exe"),
            art_type: ArtifactType::Zip,
            auth_token_source: AuthTokenSource::None
        };

        lock.add_new(tool_version);
        let tool_lock = toml::to_string(&lock).unwrap();

        toml::from_str::<ToolLock>(&tool_lock).unwrap();
    }
}
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};

use super::ConfigContainer;
use crate::config::model::ApplicationConfig;
use crate::config::model::Tokens;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ToolLock {
    wanted: BTreeMap<String, WantedVersion>,
    pub tokens: Tokens,
    definations: Vec<ToolDefinition>,
    tools: Vec<ToolVersion>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum WantedVersion {
    Latest,
    Specific(String),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub config: ApplicationConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub struct ToolVersion {
    pub name: String,
    pub version: String,
    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    pub download_url: Option<String>,
    pub exec_path: String,
    pub art_type: ArtifactType,
    pub auth_token_source: AuthTokenSource,
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub enum AuthTokenSource {
    None,
    GitHub,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ArtifactType {
    Tgz,
    Raw,
    Zip,
}

impl ToolLock {
    pub fn get_global_lock() -> Self {
        ConfigContainer::get_container_config().lock_config.unwrap()
    }

    pub fn add_new(&mut self, tool_version: ToolVersion) {
        trace!("Adding new tool: {:?}", tool_version);
        self.tools
            .retain(|x| !(x.name == tool_version.name && tool_version.version == x.version));

        self.wanted
            .entry(tool_version.name.clone())
            .or_insert(WantedVersion::Latest);
        self.tools.push(tool_version);
    }

    pub fn add_all(&mut self, tool_versions: Vec<ToolVersion>) {
        for tool in tool_versions {
            self.add_new(tool);
        }
    }

    pub fn delete_tool(&mut self, tool_name: &str) {
        self.tools.retain(|x| x.name != tool_name);
    }

    pub fn find_tool(&self, tool_name: &str) -> Vec<ToolVersion> {
        let result: Vec<ToolVersion> = self
            .tools
            .iter()
            .filter(|x| x.name == tool_name)
            .map(|x| x.clone())
            .collect();
        result
    }

    pub fn find_version(&self, tool_name: &str, version: &str) -> Option<ToolVersion> {
        self.tools
            .iter()
            .find(|x| x.name == tool_name && x.version == version)
            .map(|x| x.clone())
    }

    pub fn get_all_tools(&self) -> Vec<ToolVersion> {
        self.tools.clone()
    }

    pub fn insert_defination(&mut self, name: String, config: ApplicationConfig) {
        self.definations.push(ToolDefinition {
            name: s!(name),
            config: config.clone(),
        });
    }

    pub fn get_definations(&self) -> Vec<ToolDefinition> {
        self.definations.to_vec()
    }

    pub fn update_tokens(&mut self, tokens: &Tokens) {
        self.tokens = tokens.clone();
    }

    pub fn get_tokens(&self) -> &Tokens {
        &self.tokens
    }

    pub fn get_all_wanted(&self) -> Vec<ToolVersion> {
        self.wanted
            .keys()
            .flat_map(|name| self.get_wanted(&name))
            .collect()
    }

    pub fn get_wanted(&self, tool_name: &str) -> Option<ToolVersion> {
        let wanted = match self.wanted.get(tool_name) {
            Some(value) => value,
            None => &WantedVersion::Latest,
        };

        trace!("{} wants {:?}", tool_name, wanted);

        match wanted {
            WantedVersion::Latest => {
                let mut versions = self.find_tool(tool_name);
                versions.sort_by_key(|x| x.created_at);
                match versions.last() {
                    Some(x) => Some(x.clone()),
                    None => None,
                }
            }
            WantedVersion::Specific(version) => self.find_version(tool_name, &version),
        }
    }
}

impl std::default::Default for ToolLock {
    fn default() -> Self {
        ToolLock {
            tokens: Tokens::default(),
            tools: Vec::new(),
            wanted: BTreeMap::new(),
            definations: Vec::new(),
        }
    }
}

impl ToolVersion {
    pub fn get_download_dir(&self) -> PathBuf {
        let mut path = PathBuf::from(crate::DOWNLOAD_DIR.as_str());
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
        self.download_url.is_some()
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

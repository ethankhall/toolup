use std::collections::HashMap;
use std::cell::{RefMut, RefCell, Ref};
use std::rc::Rc;
use std::borrow::Borrow;

#[derive(Debug)]
pub struct VersionUrlResponse {
    name: String,
    created_at: i64,
    download_url: Option<String>
}

impl VersionUrlResponse {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn download_url(&self) -> Option<String> {
        self.download_url.clone()
    }
}

impl VersionUrlResponse {
    pub fn new(name: String, created_at: i64, download_url: Option<String>) -> Self {
        VersionUrlResponse { name, created_at, download_url }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct GlobalState {
    pub last_update_time: u64,
    pub tools: Box<HashMap<String, ToolGlobalState>>
}

impl GlobalState {
    pub fn get_tool<'a>(&'a self, name: &'a str) -> Option<&'a ToolGlobalState> {
        self.tools.get(name)
    }
}

impl std::default::Default for GlobalState {
    fn default() -> Self {
        GlobalState { last_update_time: 0, tools: Box::new(HashMap::new()) }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct ToolGlobalState {
    pub name: String,
    pub auth: AuthConfig,
    pub linked: Option<ToolVersion>,
    versions: Rc<RefCell<HashMap<String, ToolVersion>>>
}

impl ToolGlobalState {
    pub fn new(name: String) -> Self {
        ToolGlobalState { name, auth: AuthConfig::None, linked: None, versions: Rc::new(RefCell::new(HashMap::new())) }
    }

    pub fn get_versions(&self) -> Vec<String> {
        let versions: &RefCell<HashMap<String, ToolVersion>> = self.versions.borrow();
        let versions: Ref<HashMap<String, ToolVersion>> = versions.borrow();
        return versions.keys().map(|x| s!(x)).collect();
    }

    pub fn get_all_versions(&self) -> Vec<ToolVersion> {
        let versions: &RefCell<HashMap<String, ToolVersion>> = self.versions.borrow();
        let versions: Ref<HashMap<String, ToolVersion>> = versions.borrow();
        return versions.values().map(|x| x.clone()).collect()
    }

    pub fn get_version(&self, name: &str) -> Option<ToolVersion> {
        let versions: &RefCell<HashMap<String, ToolVersion>> = self.versions.borrow();
        let versions: Ref<HashMap<String, ToolVersion>> = versions.borrow();
        return match versions.get(name) {
            Some(value) => Some(value.clone()),
            None => None
        };
    }

    pub fn has_version(&self, name: &str) -> bool {
        let versions: &RefCell<HashMap<String, ToolVersion>> = self.versions.borrow();
        let versions: Ref<HashMap<String, ToolVersion>> = versions.borrow();
        versions.contains_key(name)
    }

    pub fn insert_version(&self, name: String, version: ToolVersion) {
        let versions: &RefCell<HashMap<String, ToolVersion>> = self.versions.borrow();
        let mut versions: RefMut<HashMap<String, ToolVersion>> = versions.borrow_mut();
        versions.insert(name, version);
    }

    pub fn get_version_to_download(&self) -> Option<DonwloadableArtifact> {
        self.get_all_versions().into_iter().filter_map(|x| {
            match x {
                ToolVersion::NoArtifact(no_art) => None,
                ToolVersion::Artifact(art) => Some(art)
            }
        }).max_by_key(|x| x.created)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum AuthConfig {
    None,
    Authorization(String)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum ToolVersion {
    NoArtifact(NoArtifact),
    Artifact(DonwloadableArtifact)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct NoArtifact {
    pub name: String,
    pub created: i64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct DonwloadableArtifact {
    pub name: String,
    pub created: i64,
    pub download_url: String,
    pub container: ArtifactType,
    pub installed: TookInstallDetails
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactType {
    Zip,
    TGZ,
    Raw
}

impl ToolVersion {
    pub fn new(url_response: VersionUrlResponse, container: ArtifactType, installed: TookInstallDetails) -> Self {
        match url_response.download_url {
            Some(url) => ToolVersion::Artifact( DonwloadableArtifact {
                name: url_response.name, 
                download_url: url,
                created: url_response.created_at,
                container: container,
                installed: installed 
            }),
            None => ToolVersion::NoArtifact(NoArtifact {
                name: url_response.name,
                created: url_response.created_at 
            })
        }
    }

    pub fn get_name(&self) -> String {
        match self {
            ToolVersion::NoArtifact(no_art) => no_art.name.clone(),
            ToolVersion::Artifact(art) => art.name.clone()
        }.to_string()
    }

    pub fn created_at(&self) -> i64 {
        match self {
            ToolVersion::NoArtifact(no_art) => no_art.created,
            ToolVersion::Artifact(art) => art.created
        }
    }

    pub fn is_downloadable(&self) -> bool {
        match self {
            ToolVersion::NoArtifact(_) => false,
            ToolVersion::Artifact(_) => true
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TookInstallDetails {
    pub exec_path: String,
    pub downloaded_at: Option<i64>
}
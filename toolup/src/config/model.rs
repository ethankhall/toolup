use std::collections::BTreeMap;

#[serde(rename_all = "kebab-case")]
#[derive(Serialize, Deserialize, Debug)]
pub struct UserConfig {
    #[serde(default)]
    #[serde(alias = "token")]
    pub tokens: Tokens,

    #[serde(alias = "tool")]
    tools: BTreeMap<String, ApplicationConfig>
}

impl UserConfig {
    pub fn tools<'a> (&'a self) -> &'a BTreeMap<String, ApplicationConfig> {
        return &self.tools
    }
}

#[serde(rename_all = "kebab-case")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tokens {
    pub github: Option<String>
}

impl Default for Tokens {
    fn default() -> Self {
        Tokens { github: None}
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ApplicationConfig {
    pub version_source: VersionSource,
    pub update_frequency: UpdateFrequency,
    pub artifact: ArtifactSource
}

impl ApplicationConfig {
    pub fn version_source<'a>(&'a self) -> &'a VersionSource {
        &self.version_source
    }

    pub fn update_frequency<'a>(&'a self) -> &'a UpdateFrequency {
        &self.update_frequency
    }

    pub fn clone(&self) -> Self {
        ApplicationConfig {
            version_source: self.version_source.clone(),
            update_frequency: self.update_frequency.clone(),
            artifact: self.artifact.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum VersionSource {
    #[serde(alias = "github")]
    GitHub { owner: String, repo: String }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ArtifactSource {
    #[serde(alias = "zip", alias = "ZIP")]
    Zip { name: String, path: String },
    #[serde(alias = "tgz", alias = "tar.gz")]
    TGZ { name: String, path: String },
    #[serde(alias = "raw")]
    Raw { name: String },
}

impl ArtifactSource {
    pub fn get_name(&self) -> String {
        match self {
            ArtifactSource::Zip { name, path: _ } => name,
            ArtifactSource::TGZ { name, path: _ } => name,
            ArtifactSource::Raw { name } => name
        }.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum UpdateFrequency {
    #[serde(alias = "fast")]
    Fast,
    #[serde(alias = "medium", alias = "med")]
    Medium,
    #[serde(alias = "slow")]
    Slow,
    #[serde(alias = "every-time", alias = "every")]
    EveryTime
}
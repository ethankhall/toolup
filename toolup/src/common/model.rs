use std::collections::BTreeMap;

#[serde(rename_all = "kebab-case")]
#[derive(Serialize, Deserialize, Debug)]
pub struct GlobalConfig {
    #[serde(default)]
    #[serde(alias = "token")]
    pub tokens: Tokens,

    #[serde(alias = "tool")]
    pub tools: BTreeMap<String, ApplicationConfig>
}

#[serde(rename_all = "kebab-case")]
#[derive(Serialize, Deserialize, Debug)]
pub struct Tokens {
    pub github: Option<String>
}

impl Default for Tokens {
    fn default() -> Self {
        Tokens { github: None}
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct ApplicationConfig {
    pub version_source: VersionSource,
    pub update_frequency: UpdateFrequency,
    pub artifact: ArtifactSource
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum VersionSource {
    #[serde(alias = "github")]
    GitHub { owner: String, repo: String}
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
use std::collections::BTreeMap;

#[serde(rename_all = "kebab-case")]
#[derive(Serialize, Deserialize, Debug)]
pub struct GlobalConfig {
    #[serde(alias = "tool")]
    pub tools: BTreeMap<String, ApplicationConfig>
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
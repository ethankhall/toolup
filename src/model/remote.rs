use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RemotePackage {
    pub name: String,
    pub update_period_seconds: i64,
    #[serde(flatten)]
    pub repository: PackageRepository,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "package-repository-type", rename_all = "kebab-case")]
pub enum PackageRepository {
    S3(S3PackageRepository),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct S3PackageRepository {
    pub url: String,
    #[serde(flatten)]
    pub auth_strategy: AuthStrategy,
}

impl fmt::Display for PackageRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageRepository::S3(s3) => write!(f, "S3 resources located at {}", s3.url),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "auth-strategy", rename_all = "kebab-case")]
pub enum AuthStrategy {
    None,
    Script(AuthScript),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AuthScript {
    pub script_path: String,
}

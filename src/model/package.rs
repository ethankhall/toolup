use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserDefinedPackage<'a> {
    pub name: &'a str,
    pub entrypoints: Vec<&'a str>,
    pub version: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratedDefinedPackage {
    pub name: String,
    pub entrypoints: BTreeMap<String, String>,
    pub version: String,
    pub achived_at: DateTime<Utc>,
    pub file_hashes: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct InstalledPackageContainer {
    pub package: GeneratedDefinedPackage,
    pub path_to_root: String,
}

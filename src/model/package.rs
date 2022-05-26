use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserDefinedPackage<'a> {
    pub name: &'a str,
    pub entrypoints: Vec<&'a str>,
    pub version: &'a str,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct GeneratedDefinedPackage {
    pub name: String,
    pub entrypoints: BTreeMap<String, String>,
    pub version: String,
    pub achived_at: DateTime<Utc>,
    pub file_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct InstalledPackageContainer {
    pub package: GeneratedDefinedPackage,
    pub path_to_root: String,
    pub remote_name: Option<String>,
    pub etag: Option<String>,
}

pub trait GenericPackage {
    fn name(&self) -> String;
    fn version(&self) -> String;
    fn id(&self) -> String;
}

impl GenericPackage for GeneratedDefinedPackage {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn version(&self) -> String {
        self.version.clone()
    }

    fn id(&self) -> String {
        crate::util::make_package_id(&self.name, &self.version)
    }
}

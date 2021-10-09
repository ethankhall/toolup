use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserDefinedPackage<'a> {
    pub name: &'a str,
    pub entrypoints: Vec<&'a str>,
    pub version: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratedDefinedPackage {
    pub name: String,
    pub entrypoints: Vec<String>,
    pub version: String,
    pub achived_at: DateTime<Utc>,
    pub file_hashes: BTreeMap<String, String>,
}
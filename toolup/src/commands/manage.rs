use clap::ArgMatches;
use regex::Regex;

use crate::common::error::*;
use crate::err;

use crate::config::ConfigContainer;
use crate::config::lock::ToolLock;
use crate::config::model::*;
use super::CliResult;

pub fn delete_tool(arg: &ArgMatches) -> CliResult {
    let name = arg.value_of("NAME").unwrap();

    let mut lock = ToolLock::get_global_lock();
    lock.delete_tool(name);
    ConfigContainer::set_lock_config(lock);

    Ok(0)
}

pub fn init(arg: &ArgMatches) -> CliResult {
    let mut lock = ToolLock::default();

    lock.update_tokens(&Tokens { github: Some(s!(arg.value_of("github_api_token").unwrap()))});
    ConfigContainer::set_lock_config(lock);
    ConfigContainer::write_config()?;

    Ok(0)
}

pub fn add_tool(arg: &ArgMatches) -> CliResult {
    let name = arg.value_of("NAME").unwrap();

    let mut lock = ToolLock::get_global_lock();
    
    let artifact_source = match (arg.value_of("raw"), arg.value_of("tgz"), arg.value_of("zip")) {
        (Some(name), _, _) => { ArtifactSource::Raw { name: s!(name) }},
        (None, Some(name), _) => { ArtifactSource::TGZ { name: s!(name), path: s!(arg.value_of("path").unwrap())}},
        (None, None, Some(name)) => {ArtifactSource::Zip { name: s!(name), path: s!(arg.value_of("path").unwrap())}},
        _ => unreachable!()
    };

    let version_source = match arg.value_of("github") {
        Some(text) => {
            let re = Regex::new("(?P<org>[0-9A-Za-z\\-_]+)/(?P<repo>[0-9A-Za-z\\-_]+)").unwrap();
            if re.is_match(text) {
                let matches = re.captures(text).unwrap();
                let org = matches.name("org").unwrap().as_str();
                let repo = matches.name("repo").unwrap().as_str();

                VersionSource::GitHub { owner: s!(org), repo: s!(repo) }
            } else {
                err!(ConfigError::GitHubRepoNotValid(s!("Given input doesn't match org/repo format")))
            }
        },
        _ => unreachable!()
    };

    lock.insert_defination(s!(name), ApplicationConfig { version_source, update_frequency: UpdateFrequency::Fast, artifact: artifact_source });

    ConfigContainer::set_lock_config(lock);
    crate::storage::pull_for_latest()?;

    Ok(0)
}
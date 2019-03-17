use chrono::{DateTime, Utc};
use json::JsonValue;

use crate::common::error::*;
use crate::config::lock::*;
use crate::err;

const GET_RELEASES: &'static str = "
query ($owner: String!, $repo: String!, $release_count:Int, $artifact_name: String) {
  repository(owner: $owner, name: $repo) {
    releases(first: $release_count, orderBy: {field: CREATED_AT, direction: DESC}) {
      nodes {
        name
        createdAt
        releaseAssets(first: 1, name: $artifact_name) {
          nodes {
            downloadUrl
          }
        }
      }
    }
  }
}";

pub enum GraphQlRequest {
    GetReleases {
        owner: String,
        repo: String,
        artifact_name: String,
        limit: i32,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum GraphQlVariables {
    GetReleases {
        owner: String,
        repo: String,
        artifact_name: String,
        release_count: i32,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct GraphQlQuery {
    query: String,
    variables: GraphQlVariables,
}

pub fn get_current_details(
    owner: String,
    repo: String,
    api_token: String,
    tool_name: &str,
    artifact: &ArtifactSource,
) -> Result<Vec<ToolVersion>, CliError> {
    let graph_api_query = make_graph_ql_body(GraphQlRequest::GetReleases {
        owner: owner,
        repo: repo,
        artifact_name: artifact.get_name(),
        limit: 15,
    });
    debug!("Body to send to GitHub: {}", graph_api_query);

    let client = reqwest::Client::new();
    let url: &str = &format!("{}/graphql", get_github_api());
    let res = client
        .post(url)
        .body(graph_api_query)
        .header("Authorization", s!(api_token))
        .send();

    let body: String = match res {
        Ok(mut response) => {
            if response.status().is_success() {
                response.text().unwrap()
            } else {
                match response.text() {
                    Ok(text) => err!(ApiError::CallWasNotSuccessful(text)),
                    Err(e) => err!(ApiError::CallWasNotSuccessful(e.to_string())),
                }
            }
        }
        Err(e) => err!(ApiError::UnableToContactGitHub(e.to_string())),
    };

    let body = json::parse(&body).expect("JSON from GitHub should be valid");

    debug!("Response from GitHub: {}", body);

    parse_get_release_response(body, tool_name, artifact)
}

fn parse_get_release_response(
    body: JsonValue,
    tool_name: &str,
    artifact: &ArtifactSource,
) -> Result<Vec<ToolVersion>, CliError> {
    let (art_type, exec_path) = match artifact {
        ArtifactSource::Zip { name: _, path } => (ArtifactType::Zip, path),
        ArtifactSource::TGZ { name: _, path } => (ArtifactType::Tgz, path),
        ArtifactSource::Raw { name } => (ArtifactType::Raw, name),
    };

    if let JsonValue::Object(object) = body {
        match &object["data"] {
            JsonValue::Object(data) => {
                if let JsonValue::Object(repo) = &data["repository"] {
                    if let JsonValue::Object(releases) = &repo["releases"] {
                        if let JsonValue::Array(nodes) = &releases["nodes"] {
                            return Ok(nodes
                                .into_iter()
                                .map(|x| {
                                    let path = get_artifact_url(&x);

                                    let name = match &x["name"] {
                                        JsonValue::Short(name) => s!(name),
                                        JsonValue::String(name) => s!(name),
                                        _ => return None,
                                    };

                                    let created_at = match &x["createdAt"] {
                                        JsonValue::Short(name) => s!(name),
                                        JsonValue::String(name) => s!(name),
                                        _ => return None,
                                    };

                                    let created_at =
                                        DateTime::parse_from_rfc3339(&created_at).unwrap();

                                    Some(ToolVersion {
                                        name: s!(tool_name),
                                        version: name,
                                        created_at: created_at.with_timezone(&Utc),
                                        download_url: path,
                                        exec_path: s!(exec_path),
                                        art_type: art_type.clone(),
                                        auth_token_source: AuthTokenSource::GitHub,
                                    })
                                })
                                .filter(|x| x.is_some())
                                .map(|x| x.unwrap())
                                .collect());
                        }
                    }
                }
            }
            _ => {
                let error_message = if let JsonValue::Array(errors) = &object["errors"] {
                    let error_messages: Vec<String> = errors
                        .into_iter()
                        .map(|x| {
                            if let JsonValue::Object(error) = x {
                                error["message"].to_string()
                            } else {
                                s!("Unable to parse error response")
                            }
                        })
                        .collect();

                    error_messages.join(", ")
                } else {
                    s!("Unable to parse GitHub error response")
                };

                err!(ApiError::CallWasNotSuccessful(error_message))
            }
        }
    }

    err!(ApiError::CallWasNotSuccessful(s!(
        "Unable to parse GitHub error response"
    )))
}

fn get_artifact_url(release: &JsonValue) -> Option<String> {
    if let JsonValue::Object(assets) = &release["releaseAssets"] {
        if let JsonValue::Array(nodes) = &assets["nodes"] {
            if nodes.is_empty() {
                return None;
            }

            if let JsonValue::Object(node) = &nodes[0] {
                return match &node["downloadUrl"] {
                    JsonValue::Short(name) => Some(s!(name)),
                    JsonValue::String(name) => Some(s!(name)),
                    _ => None,
                };
            }
        }
    }

    return None;
}

fn make_graph_ql_body(vars: GraphQlRequest) -> String {
    let query: GraphQlQuery = match vars {
        GraphQlRequest::GetReleases {
            owner,
            repo,
            artifact_name,
            limit,
        } => GraphQlQuery {
            query: GET_RELEASES.replace("\n", ""),
            variables: GraphQlVariables::GetReleases {
                owner,
                repo,
                artifact_name,
                release_count: limit,
            },
        },
    };

    return serde_json::ser::to_string(&query).expect("To be able to make JSON from model");
}

pub fn get_github_api() -> String {
    use std::env;

    match env::var("GITHUB_API_SERVER") {
        Ok(value) => value,
        Err(_) => s!("https://api.github.com"),
    }
}

#[cfg(test)]
pub fn get_github_token(_tokens: &Tokens) -> Result<String, CliError> {
    warn!("Using debug GITHUB headers!");
    Ok(s!("TEST"))
}

#[cfg(not(test))]
pub fn get_github_token(tokens: &Tokens) -> Result<String, CliError> {
    match &tokens.github {
        Some(value) => Ok(format!("bearer {}", value)),
        None => err!(ApiError::GitHubTokenNotProvided),
    }
}

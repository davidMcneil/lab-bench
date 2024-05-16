use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use chrono::{DateTime, TimeDelta, Utc};
use futures::future::join_all;
use percent_encoding::NON_ALPHANUMERIC;
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize};
use strum::Display;
use tracing::{error, info};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrderBy {
    #[default]
    CreatedAt,
    UpdatedAt,
    Title,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    Asc,
    #[default]
    Desc,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    #[default]
    CreatedByMe,
    AssignedToMe,
    All,
}

#[derive(Clone, Copy, Debug, Default, Display, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum State {
    Opened,
    Closed,
    Locked,
    Merged,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Wip {
    Yes,
    No,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum MergeRequestsDomain {
    AuthorUsername(String),
    ProjectPath(String),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MergeRequestsQuery {
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub order_by: OrderBy,
    pub scope: Scope,
    pub sort: Sort,
    pub state: Option<State>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub wip: Option<Wip>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct MergeRequest {
    pub author: User,
    pub blocking_discussions_resolved: bool,
    pub created_at: DateTime<Utc>,
    pub detailed_merge_status: MergeStatus,
    pub draft: bool,
    pub has_conflicts: bool,
    pub head_pipeline: Option<Pipeline>,
    pub id: i64,
    pub iid: i64,
    pub latest_build_finished_at: Option<DateTime<Utc>>,
    pub latest_build_started_at: Option<DateTime<Utc>>,
    pub merge_commit_sha: Option<String>,
    pub merge_user: Option<User>,
    pub merge_when_pipeline_succeeds: bool,
    pub merged_at: Option<DateTime<Utc>>,
    pub project_id: i64,
    pub references: References,
    #[serde(default)]
    pub reviewers: Vec<User>,
    pub sha: Option<String>,
    pub source_branch: String,
    pub state: State,
    pub title: String,
    pub updated_at: DateTime<Utc>,
    pub user_notes_count: i64,
    pub web_url: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct User {
    pub avatar_url: String,
    pub id: i64,
    pub name: String,
    pub username: String,
    pub state: String,
    pub web_url: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Pipeline {
    pub id: i64,
    pub sha: String,
    pub status: PipelineStatus,
    pub web_url: String,
    #[serde(deserialize_with = "deserialize_time_delta_from_seconds_with_default")]
    pub duration: TimeDelta,
    #[serde(deserialize_with = "deserialize_time_delta_from_seconds_with_default")]
    pub queued_duration: TimeDelta,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct References {
    pub full: String,
    pub short: String,
    pub relative: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Display, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MergeStatus {
    /// Blocked by another merge request.
    #[serde(alias = "merge_request_blocked")]
    BlockedStatus,
    /// Git is testing if a valid merge is possible.
    Checking,
    /// Git has not yet tested if a valid merge is possible.
    Unchecked,
    /// A CI/CD pipeline must succeed before merge.
    CiMustPass,
    /// A CI/CD pipeline is still running.
    CiStillRunning,
    /// All discussions must be resolved before merge.
    DiscussionsNotResolved,
    /// Can’t merge because the merge request is a draft.
    DraftStatus,
    /// All status checks must pass before merge.
    ExternalStatusChecks,
    /// The branch can merge cleanly into the target branch.
    Mergeable,
    /// Approval is required before merge.
    NotApproved,
    /// The merge request must be open before merge.
    NotOpen,
    /// The title or description must reference a Jira issue.
    JiraAssociationMissing,
    /// The merge request must be rebased.
    NeedRebase,
    /// There are conflicts between the source and target branches.
    Conflict,
    /// The merge request has reviewers who have requested changes.
    RequestedChanges,
    /// Not documented in gitlab
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Default, Deserialize, Display, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PipelineStatus {
    Created,
    WaitingForResource,
    Preparing,
    Pending,
    Running,
    Success,
    Failed,
    Canceled,
    Skipped,
    Manual,
    Scheduled,
    /// Not documented in gitlab
    #[serde(other)]
    #[default]
    Unknown,
}

/// Fetch merge request from query params and a list of domains
pub async fn fetch_merge_requests(
    gitlab_url: &str,
    private_token: &str,
    query: &MergeRequestsQuery,
    domains: &[MergeRequestsDomain],
) -> Result<Vec<MergeRequest>> {
    let futures = domains
        .iter()
        .map(|domain| fetch_merge_requests_helper(gitlab_url, private_token, query, domain));
    let results = join_all(futures).await;
    // TODO: sort the results
    Ok(results
        .into_iter()
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect())
}

/// Fetch merge requests individually to get the full data (ie pipeline)
pub async fn fetch_merge_requests_with_full_data(
    gitlab_url: &str,
    private_token: &str,
    merge_requests: &[MergeRequest],
) -> Result<Vec<MergeRequest>> {
    let futures = merge_requests
        .iter()
        .map(|mr| fetch_merge_request_no_fail(gitlab_url, private_token, mr));
    let results = join_all(futures).await;
    Ok(results.into_iter().collect::<Vec<_>>())
}

async fn fetch_merge_requests_helper(
    gitlab_url: &str,
    private_token: &str,
    query: &MergeRequestsQuery,
    domain: &MergeRequestsDomain,
) -> Result<Vec<MergeRequest>> {
    info!("fetching merge requests with query {:?}", query);
    info!("domain {:?}", domain);

    let request = client();

    let request = match domain {
        MergeRequestsDomain::AuthorUsername(author_username) => request
            .get(format!("{gitlab_url}/merge_requests"))
            .query(&[("author_username", author_username)]),
        MergeRequestsDomain::ProjectPath(project_path) => {
            let project_path =
                percent_encoding::utf8_percent_encode(project_path, NON_ALPHANUMERIC);
            request.get(format!(
                "{gitlab_url}/projects/{project_path}/merge_requests",
            ))
        }
    };

    let response = request
        .header("PRIVATE-TOKEN", private_token)
        .query(&query)
        .send()
        .await?;
    let merge_requests = if response.status().is_success() {
        response.json::<Vec<MergeRequest>>().await?
    } else {
        return Err(anyhow!(
            "fetching merge requests failed with status {}",
            response.status()
        ));
    };
    info!("fetched {} merge requests", merge_requests.len());
    Ok(merge_requests)
}

/// If fetching a single merge request fails just swallow the error and return a copy of the
/// supplied merge request
async fn fetch_merge_request_no_fail(
    gitlab_url: &str,
    private_token: &str,
    merge_request: &MergeRequest,
) -> MergeRequest {
    fetch_merge_request(gitlab_url, private_token, merge_request)
        .await
        .ok()
        .unwrap_or_else(|| merge_request.clone())
}

async fn fetch_merge_request(
    gitlab_url: &str,
    private_token: &str,
    merge_request: &MergeRequest,
) -> Result<MergeRequest> {
    let full = &merge_request.references.full;

    let project_id = merge_request.project_id;
    let merge_request_iid = merge_request.iid;

    let response = client()
        .get(format!(
            "{gitlab_url}/projects/{project_id}/merge_requests/{merge_request_iid}",
        ))
        .header("PRIVATE-TOKEN", private_token)
        .send()
        .await?;
    let merge_request = if response.status().is_success() {
        response
            .json::<MergeRequest>()
            .await
            .inspect_err(|e| error!("failed fetching merge request {full}: {e}"))?
    } else {
        return Err(anyhow!(
            "fetching merge requests failed with status {}",
            response.status()
        ));
    };

    Ok(merge_request)
}

fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| Client::new())
}

fn deserialize_time_delta_from_seconds_with_default<'de, D>(
    deserializer: D,
) -> Result<TimeDelta, D::Error>
where
    D: Deserializer<'de>,
{
    let seconds: Option<i64> = Deserialize::deserialize(deserializer)?;
    Ok(TimeDelta::seconds(seconds.unwrap_or_default()))
}

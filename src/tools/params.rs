use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchIssuesParams {
    /// JQL query string (e.g., 'project = PROJ AND status = Open')
    pub jql: String,
    /// Maximum number of results to return (default: 50, max: 100)
    pub max_results: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetIssueParams {
    /// The issue key (e.g., 'PROJ-123')
    pub issue_key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddCommentParams {
    /// The issue key (e.g., 'PROJ-123')
    pub issue_key: String,
    /// The comment text to add to the issue
    pub comment: String,
}

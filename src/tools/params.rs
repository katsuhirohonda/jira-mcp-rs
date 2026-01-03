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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateIssueParams {
    /// The issue key (e.g., 'PROJ-123')
    pub issue_key: String,
    /// New summary/title for the issue
    pub summary: Option<String>,
    /// Due date in YYYY-MM-DD format (e.g., '2025-01-31')
    pub due_date: Option<String>,
    /// Priority name (e.g., 'High', 'Medium', 'Low')
    pub priority: Option<String>,
    /// Assignee's account ID
    pub assignee_account_id: Option<String>,
    /// Parent issue key for subtasks or epic (e.g., 'EPIC-123')
    pub parent_key: Option<String>,
    /// Labels to set on the issue
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetEpicsParams {
    /// The project key (e.g., 'PROJ')
    pub project_key: String,
    /// Maximum number of results to return (default: 50, max: 100)
    pub max_results: Option<u32>,
}

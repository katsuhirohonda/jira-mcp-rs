use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub jql: String,
    pub max_results: u32,
    pub fields: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub total: Option<u32>,
    pub max_results: Option<u32>,
    pub start_at: Option<u32>,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Issue {
    pub id: String,
    pub key: String,
    #[serde(rename = "self")]
    pub self_url: String,
    pub fields: IssueFields,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IssueFields {
    pub summary: Option<String>,
    pub status: Option<Status>,
    pub assignee: Option<User>,
    pub priority: Option<Priority>,
    #[serde(rename = "issuetype")]
    pub issue_type: Option<IssueType>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub description: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IssueType {
    pub name: String,
    pub subtask: bool,
}

/// Response from GET /rest/api/2/issue/{issueIdOrKey}/comment
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentResponse {
    pub start_at: u32,
    pub max_results: u32,
    pub total: u32,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Status {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub display_name: String,
    pub email_address: Option<String>,
    #[serde(rename = "accountId")]
    pub account_id: Option<String>, // Account ID is optional as some users (like apps) might not have it in the same context, or for backward compatibility
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Priority {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AddCommentRequest {
    pub body: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Comment {
    pub id: String,
    #[serde(rename = "self")]
    pub self_url: String,
    pub author: Option<User>,
    pub created: Option<String>,
    pub body: Option<serde_json::Value>,
}

/// Request body for updating an issue.
/// Uses HashMap to allow flexible field updates.
#[derive(Debug, Serialize, Default)]
pub struct UpdateIssueRequest {
    /// Fields to update (e.g., "summary", "duedate", "priority", "assignee", "parent")
    pub fields: HashMap<String, serde_json::Value>,
}

impl UpdateIssueRequest {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the summary (title) of the issue
    pub fn summary(mut self, summary: &str) -> Self {
        self.fields
            .insert("summary".to_string(), serde_json::json!(summary));
        self
    }

    /// Set the due date (format: "YYYY-MM-DD")
    pub fn due_date(mut self, date: &str) -> Self {
        self.fields
            .insert("duedate".to_string(), serde_json::json!(date));
        self
    }

    /// Set the priority by name (e.g., "High", "Medium", "Low")
    pub fn priority(mut self, priority_name: &str) -> Self {
        self.fields.insert(
            "priority".to_string(),
            serde_json::json!({"name": priority_name}),
        );
        self
    }

    /// Set the assignee by account ID
    pub fn assignee(mut self, account_id: &str) -> Self {
        self.fields.insert(
            "assignee".to_string(),
            serde_json::json!({"accountId": account_id}),
        );
        self
    }

    /// Set the parent issue (for subtasks or moving to an epic)
    pub fn parent(mut self, parent_key: &str) -> Self {
        self.fields.insert(
            "parent".to_string(),
            serde_json::json!({"key": parent_key}),
        );
        self
    }

    /// Set labels
    pub fn labels(mut self, labels: Vec<&str>) -> Self {
        self.fields
            .insert("labels".to_string(), serde_json::json!(labels));
        self
    }
}

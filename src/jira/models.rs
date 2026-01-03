use serde::{Deserialize, Serialize};

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
    pub total: u32,
    pub max_results: u32,
    pub start_at: u32,
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
    pub created: Option<String>,
    pub updated: Option<String>,
    pub description: Option<serde_json::Value>,
    pub comment: Option<CommentList>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentList {
    pub comments: Vec<Comment>,
    pub total: u32,
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
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Priority {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AddCommentRequest {
    pub body: CommentBody,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommentBody {
    #[serde(rename = "type")]
    pub doc_type: String,
    pub version: u32,
    pub content: Vec<CommentParagraph>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommentParagraph {
    #[serde(rename = "type")]
    pub paragraph_type: String,
    pub content: Vec<CommentText>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommentText {
    #[serde(rename = "type")]
    pub text_type: String,
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Comment {
    pub id: String,
    #[serde(rename = "self")]
    pub self_url: String,
    pub author: Option<User>,
    pub created: Option<String>,
    pub body: Option<CommentBody>,
}

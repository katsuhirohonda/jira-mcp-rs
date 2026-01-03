use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct JiraClient {
    client: Client,
    base_url: String,
    auth_header: String,
}

impl JiraClient {
    pub fn new(base_url: &str, email: &str, api_token: &str) -> Self {
        let credentials = format!("{}:{}", email, api_token);
        let auth_header = format!("Basic {}", STANDARD.encode(credentials));

        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_header,
        }
    }

    pub async fn search_issues(&self, jql: &str, max_results: u32) -> Result<SearchResult> {
        let url = format!("{}/rest/api/3/search/jql", self.base_url);

        let request_body = SearchRequest {
            jql: jql.to_string(),
            max_results,
            fields: vec![
                "summary".to_string(),
                "status".to_string(),
                "assignee".to_string(),
                "priority".to_string(),
                "created".to_string(),
                "updated".to_string(),
            ],
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Jira API error ({}): {}", status, error_text);
        }

        let result = response.json::<SearchResult>().await?;
        Ok(result)
    }

    pub async fn get_issue(&self, issue_key: &str) -> Result<Issue> {
        let url = format!("{}/rest/api/3/issue/{}", self.base_url, issue_key);

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Jira API error ({}): {}", status, error_text);
        }

        let issue = response.json::<Issue>().await?;
        Ok(issue)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchRequest {
    jql: String,
    max_results: u32,
    fields: Vec<String>,
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

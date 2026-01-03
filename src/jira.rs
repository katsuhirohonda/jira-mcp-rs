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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_issue(key: &str, summary: &str, status: &str) -> Issue {
        Issue {
            id: "10001".to_string(),
            key: key.to_string(),
            self_url: format!("https://example.atlassian.net/rest/api/3/issue/{}", key),
            fields: IssueFields {
                summary: Some(summary.to_string()),
                status: Some(Status {
                    name: status.to_string(),
                }),
                assignee: Some(User {
                    display_name: "Test User".to_string(),
                    email_address: Some("test@example.com".to_string()),
                }),
                priority: Some(Priority {
                    name: "Medium".to_string(),
                }),
                created: Some("2024-01-15T10:00:00.000+0000".to_string()),
                updated: Some("2024-01-16T14:30:00.000+0000".to_string()),
                description: None,
            },
        }
    }

    #[tokio::test]
    async fn search_issues_returns_matching_issues() {
        // Given: a mock Jira server with issues
        let mock_server = MockServer::start().await;
        let expected_issue = create_test_issue("PROJ-123", "Fix login bug", "Open");
        let response_body = SearchResult {
            total: 1,
            max_results: 50,
            start_at: 0,
            issues: vec![expected_issue],
        };

        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .and(header("Authorization", "Basic dGVzdEBleGFtcGxlLmNvbTp0ZXN0LXRva2Vu"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        // When: searching for issues
        let result = client.search_issues("project = PROJ", 50).await.unwrap();

        // Then: the matching issues are returned
        assert_eq!(result.total, 1);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].key, "PROJ-123");
        assert_eq!(
            result.issues[0].fields.summary.as_deref(),
            Some("Fix login bug")
        );
    }

    #[tokio::test]
    async fn search_issues_returns_empty_when_no_matches() {
        // Given: a mock server returning no issues
        let mock_server = MockServer::start().await;
        let response_body = SearchResult {
            total: 0,
            max_results: 50,
            start_at: 0,
            issues: vec![],
        };

        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        // When: searching with no matches
        let result = client.search_issues("project = EMPTY", 50).await.unwrap();

        // Then: an empty result is returned
        assert_eq!(result.total, 0);
        assert!(result.issues.is_empty());
    }

    #[tokio::test]
    async fn search_issues_returns_error_on_api_failure() {
        // Given: a mock server returning an error
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "bad@example.com", "invalid-token");

        // When: searching with invalid credentials
        let result = client.search_issues("project = PROJ", 50).await;

        // Then: an error is returned
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("401"));
    }

    #[tokio::test]
    async fn get_issue_returns_issue_details() {
        // Given: a mock server with a specific issue
        let mock_server = MockServer::start().await;
        let expected_issue = create_test_issue("PROJ-456", "Implement feature X", "In Progress");

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/PROJ-456"))
            .and(header("Authorization", "Basic dGVzdEBleGFtcGxlLmNvbTp0ZXN0LXRva2Vu"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_issue))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        // When: getting a specific issue
        let issue = client.get_issue("PROJ-456").await.unwrap();

        // Then: the issue details are returned
        assert_eq!(issue.key, "PROJ-456");
        assert_eq!(
            issue.fields.summary.as_deref(),
            Some("Implement feature X")
        );
        assert_eq!(
            issue.fields.status.as_ref().map(|s| s.name.as_str()),
            Some("In Progress")
        );
    }

    #[tokio::test]
    async fn get_issue_returns_error_when_not_found() {
        // Given: a mock server returning 404
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/PROJ-999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Issue not found"))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        // When: getting a non-existent issue
        let result = client.get_issue("PROJ-999").await;

        // Then: an error is returned
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("404"));
    }

    #[test]
    fn client_trims_trailing_slash_from_base_url() {
        // Given: a base URL with trailing slash
        let client = JiraClient::new(
            "https://example.atlassian.net/",
            "test@example.com",
            "token",
        );

        // Then: the trailing slash is removed
        assert_eq!(client.base_url, "https://example.atlassian.net");
    }

    #[test]
    fn client_generates_correct_auth_header() {
        // Given: credentials
        let client = JiraClient::new(
            "https://example.atlassian.net",
            "user@example.com",
            "api-token",
        );

        // Then: the auth header is correctly encoded
        // base64("user@example.com:api-token") = "dXNlckBleGFtcGxlLmNvbTphcGktdG9rZW4="
        assert_eq!(
            client.auth_header,
            "Basic dXNlckBleGFtcGxlLmNvbTphcGktdG9rZW4="
        );
    }
}

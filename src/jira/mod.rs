mod models;

pub use models::*;

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::Client;

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
                "issuetype".to_string(),
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

    /// Update an issue's fields.
    ///
    /// # Example
    /// ```ignore
    /// let update = UpdateIssueRequest::new()
    ///     .due_date("2025-01-31")
    ///     .priority("High")
    ///     .parent("EPIC-123");
    ///
    /// client.update_issue("PROJ-456", update).await?;
    /// ```
    pub async fn update_issue(&self, issue_key: &str, update: UpdateIssueRequest) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}", self.base_url, issue_key);

        let response = self
            .client
            .put(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&update)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Jira API error ({}): {}", status, error_text);
        }

        Ok(())
    }

    /// Get comments for an issue.
    ///
    /// Uses the dedicated comment endpoint for better pagination support.
    /// Reference: https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-comments/
    ///
    /// # Example
    /// ```ignore
    /// let comments = client.get_comments("PROJ-123", 0, 50).await?;
    /// for comment in comments.comments {
    ///     println!("{}: {:?}", comment.id, comment.body);
    /// }
    /// ```
    pub async fn get_comments(
        &self,
        issue_key: &str,
        start_at: u32,
        max_results: u32,
    ) -> Result<CommentResponse> {
        let url = format!(
            "{}/rest/api/3/issue/{}/comment?startAt={}&maxResults={}",
            self.base_url, issue_key, start_at, max_results
        );

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

        let result = response.json::<CommentResponse>().await?;
        Ok(result)
    }

    pub async fn add_comment(&self, issue_key: &str, comment: &str) -> Result<Comment> {
        let url = format!("{}/rest/api/3/issue/{}/comment", self.base_url, issue_key);

        let request_body = AddCommentRequest {
            body: serde_json::json!({
                "type": "doc",
                "version": 1,
                "content": [
                    {
                        "type": "paragraph",
                        "content": [
                            {
                                "type": "text",
                                "text": comment
                            }
                        ]
                    }
                ]
            }),
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

        let comment = response.json::<Comment>().await?;
        Ok(comment)
    }
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
                    account_id: Some("test-account-id".to_string()),
                }),
                priority: Some(Priority {
                    name: "Medium".to_string(),
                }),
                created: Some("2024-01-15T10:00:00.000+0000".to_string()),
                updated: Some("2024-01-16T14:30:00.000+0000".to_string()),
                description: None,
                issue_type: Some(IssueType {
                    name: "Story".to_string(),
                    subtask: false,
                }),
            },
        }
    }

    #[tokio::test]
    async fn search_issues_returns_matching_issues() {
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
            .and(header(
                "Authorization",
                "Basic dGVzdEBleGFtcGxlLmNvbTp0ZXN0LXRva2Vu",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let result = client.search_issues("project = PROJ", 50).await.unwrap();

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

        let result = client.search_issues("project = EMPTY", 50).await.unwrap();

        assert_eq!(result.total, 0);
        assert!(result.issues.is_empty());
    }

    #[tokio::test]
    async fn search_issues_returns_error_on_api_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "bad@example.com", "invalid-token");

        let result = client.search_issues("project = PROJ", 50).await;

        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("401"));
    }

    #[tokio::test]
    async fn get_issue_returns_issue_details() {
        let mock_server = MockServer::start().await;
        let expected_issue = create_test_issue("PROJ-456", "Implement feature X", "In Progress");

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/PROJ-456"))
            .and(header(
                "Authorization",
                "Basic dGVzdEBleGFtcGxlLmNvbTp0ZXN0LXRva2Vu",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_issue))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let issue = client.get_issue("PROJ-456").await.unwrap();

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
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/PROJ-999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Issue not found"))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let result = client.get_issue("PROJ-999").await;

        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("404"));
    }

    #[tokio::test]
    async fn add_comment_creates_comment_on_issue() {
        let mock_server = MockServer::start().await;
        let response_body = Comment {
            id: "10100".to_string(),
            self_url: "https://example.atlassian.net/rest/api/3/issue/PROJ-123/comment/10100"
                .to_string(),
            author: Some(User {
                display_name: "Test User".to_string(),
                email_address: Some("test@example.com".to_string()),
                account_id: Some("test-account-id".to_string()),
            }),
            created: Some("2024-01-17T09:00:00.000+0000".to_string()),
            body: None,
        };

        Mock::given(method("POST"))
            .and(path("/rest/api/3/issue/PROJ-123/comment"))
            .and(header(
                "Authorization",
                "Basic dGVzdEBleGFtcGxlLmNvbTp0ZXN0LXRva2Vu",
            ))
            .respond_with(ResponseTemplate::new(201).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let comment = client
            .add_comment("PROJ-123", "This is a test comment")
            .await
            .unwrap();

        assert_eq!(comment.id, "10100");
        assert_eq!(
            comment.author.as_ref().map(|a| a.display_name.as_str()),
            Some("Test User")
        );
    }

    #[tokio::test]
    async fn add_comment_returns_error_when_issue_not_found() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/rest/api/3/issue/PROJ-999/comment"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Issue not found"))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let result = client.add_comment("PROJ-999", "Test comment").await;

        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("404"));
    }

    #[tokio::test]
    async fn update_issue_updates_fields_successfully() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/rest/api/3/issue/PROJ-123"))
            .and(header(
                "Authorization",
                "Basic dGVzdEBleGFtcGxlLmNvbTp0ZXN0LXRva2Vu",
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let update = UpdateIssueRequest::new()
            .due_date("2025-02-01")
            .priority("High");

        let result = client.update_issue("PROJ-123", update).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_issue_with_parent_epic() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/rest/api/3/issue/PROJ-456"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let update = UpdateIssueRequest::new().parent("EPIC-100");

        let result = client.update_issue("PROJ-456", update).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_issue_returns_error_when_issue_not_found() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/rest/api/3/issue/PROJ-999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Issue not found"))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let update = UpdateIssueRequest::new().summary("New summary");

        let result = client.update_issue("PROJ-999", update).await;

        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("404"));
    }

    #[test]
    fn client_trims_trailing_slash_from_base_url() {
        let client = JiraClient::new(
            "https://example.atlassian.net/",
            "test@example.com",
            "token",
        );

        assert_eq!(client.base_url, "https://example.atlassian.net");
    }

    #[test]
    fn client_generates_correct_auth_header() {
        let client = JiraClient::new(
            "https://example.atlassian.net",
            "user@example.com",
            "api-token",
        );

        assert_eq!(
            client.auth_header,
            "Basic dXNlckBleGFtcGxlLmNvbTphcGktdG9rZW4="
        );
    }

    #[tokio::test]
    async fn get_comments_returns_paginated_comments() {
        let mock_server = MockServer::start().await;
        let response_body = CommentResponse {
            start_at: 0,
            max_results: 50,
            total: 2,
            comments: vec![
                Comment {
                    id: "10001".to_string(),
                    self_url: "https://example.atlassian.net/rest/api/2/issue/PROJ-123/comment/10001"
                        .to_string(),
                    author: Some(User {
                        display_name: "Alice".to_string(),
                        email_address: Some("alice@example.com".to_string()),
                        account_id: Some("alice-account-id".to_string()),
                    }),
                    created: Some("2024-01-15T10:00:00.000+0000".to_string()),
                    body: Some(serde_json::json!({
                        "type": "doc",
                        "version": 1,
                        "content": [{
                            "type": "paragraph",
                            "content": [{"type": "text", "text": "First comment"}]
                        }]
                    })),
                },
                Comment {
                    id: "10002".to_string(),
                    self_url: "https://example.atlassian.net/rest/api/2/issue/PROJ-123/comment/10002"
                        .to_string(),
                    author: Some(User {
                        display_name: "Bob".to_string(),
                        email_address: Some("bob@example.com".to_string()),
                        account_id: Some("bob-account-id".to_string()),
                    }),
                    created: Some("2024-01-16T14:00:00.000+0000".to_string()),
                    body: Some(serde_json::json!({
                        "type": "doc",
                        "version": 1,
                        "content": [{
                            "type": "paragraph",
                            "content": [{"type": "text", "text": "Second comment"}]
                        }]
                    })),
                },
            ],
        };

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/PROJ-123/comment"))
            .and(header(
                "Authorization",
                "Basic dGVzdEBleGFtcGxlLmNvbTp0ZXN0LXRva2Vu",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let result = client.get_comments("PROJ-123", 0, 50).await.unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.comments.len(), 2);
        assert_eq!(result.comments[0].id, "10001");
        assert_eq!(result.comments[1].id, "10002");
    }

    #[tokio::test]
    async fn get_comments_returns_empty_when_no_comments() {
        let mock_server = MockServer::start().await;
        let response_body = CommentResponse {
            start_at: 0,
            max_results: 50,
            total: 0,
            comments: vec![],
        };

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/PROJ-456/comment"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let result = client.get_comments("PROJ-456", 0, 50).await.unwrap();

        assert_eq!(result.total, 0);
        assert!(result.comments.is_empty());
    }

    #[tokio::test]
    async fn get_comments_returns_error_when_issue_not_found() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/PROJ-999/comment"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Issue not found"))
            .mount(&mock_server)
            .await;

        let client = JiraClient::new(&mock_server.uri(), "test@example.com", "test-token");

        let result = client.get_comments("PROJ-999", 0, 50).await;

        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("404"));
    }
}

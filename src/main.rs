mod jira;

use anyhow::Result;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    handler::server::tool::Parameters,
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError,
    ServiceExt,
};
use serde::Deserialize;
use std::sync::Arc;
use jira::JiraClient;

#[derive(Clone)]
pub struct JiraServer {
    jira: Arc<JiraClient>,
    tool_router: ToolRouter<Self>,
}

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

#[tool_router]
impl JiraServer {
    fn new(jira: JiraClient) -> Self {
        Self {
            jira: Arc::new(jira),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Search for Jira issues using JQL (Jira Query Language). Returns a list of issues matching the query.")]
    async fn search_issues(
        &self,
        Parameters(params): Parameters<SearchIssuesParams>,
    ) -> Result<CallToolResult, McpError> {
        let max_results = params.max_results.unwrap_or(50).min(100);

        match self.jira.search_issues(&params.jql, max_results).await {
            Ok(result) => {
                let output = format_search_result(&result);
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to search issues: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Get detailed information about a specific Jira issue by its key (e.g., PROJ-123).")]
    async fn get_issue(
        &self,
        Parameters(params): Parameters<GetIssueParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.jira.get_issue(&params.issue_key).await {
            Ok(issue) => {
                let output = format_issue(&issue);
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get issue: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Add a comment to a Jira issue. Use this to leave notes, updates, or feedback on an issue.")]
    async fn add_comment(
        &self,
        Parameters(params): Parameters<AddCommentParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.jira.add_comment(&params.issue_key, &params.comment).await {
            Ok(comment) => {
                let output = format_comment(&params.issue_key, &comment);
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to add comment: {}",
                e
            ))])),
        }
    }
}

#[tool_handler]
impl rmcp::ServerHandler for JiraServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Jira MCP Server - Search, retrieve, and comment on Jira issues".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

fn format_search_result(result: &jira::SearchResult) -> String {
    let mut output = format!(
        "Found {} issues (showing {} of {}):\n\n",
        result.total,
        result.issues.len(),
        result.total
    );

    for issue in &result.issues {
        let status = issue
            .fields
            .status
            .as_ref()
            .map(|s| s.name.as_str())
            .unwrap_or("Unknown");
        let summary = issue
            .fields
            .summary
            .as_deref()
            .unwrap_or("No summary");
        let assignee = issue
            .fields
            .assignee
            .as_ref()
            .map(|a| a.display_name.as_str())
            .unwrap_or("Unassigned");

        output.push_str(&format!(
            "- **{}** [{}] {}\n  Assignee: {}\n\n",
            issue.key, status, summary, assignee
        ));
    }

    output
}

fn format_comment(issue_key: &str, comment: &jira::Comment) -> String {
    let author = comment
        .author
        .as_ref()
        .map(|a| a.display_name.as_str())
        .unwrap_or("Unknown");
    let created = comment.created.as_deref().unwrap_or("Unknown");

    format!(
        r#"Comment added successfully to {}

**Comment ID:** {}
**Author:** {}
**Created:** {}
"#,
        issue_key, comment.id, author, created
    )
}

fn format_issue(issue: &jira::Issue) -> String {
    let status = issue
        .fields
        .status
        .as_ref()
        .map(|s| s.name.as_str())
        .unwrap_or("Unknown");
    let summary = issue
        .fields
        .summary
        .as_deref()
        .unwrap_or("No summary");
    let assignee = issue
        .fields
        .assignee
        .as_ref()
        .map(|a| a.display_name.as_str())
        .unwrap_or("Unassigned");
    let priority = issue
        .fields
        .priority
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("None");
    let created = issue.fields.created.as_deref().unwrap_or("Unknown");
    let updated = issue.fields.updated.as_deref().unwrap_or("Unknown");

    format!(
        r#"# {} - {}

**Status:** {}
**Assignee:** {}
**Priority:** {}
**Created:** {}
**Updated:** {}
**URL:** {}
"#,
        issue.key, summary, status, assignee, priority, created, updated, issue.self_url
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let base_url = std::env::var("JIRA_BASE_URL")
        .expect("JIRA_BASE_URL environment variable is required");
    let email = std::env::var("JIRA_EMAIL")
        .expect("JIRA_EMAIL environment variable is required");
    let api_token = std::env::var("JIRA_API_TOKEN")
        .expect("JIRA_API_TOKEN environment variable is required");

    let jira = JiraClient::new(&base_url, &email, &api_token);
    let server = JiraServer::new(jira);

    tracing::info!("Starting Jira MCP server...");

    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jira::{Issue, IssueFields, Priority, SearchResult, Status, User};

    fn create_test_issue(key: &str, summary: &str, status: &str, assignee: &str) -> Issue {
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
                    display_name: assignee.to_string(),
                    email_address: Some("test@example.com".to_string()),
                }),
                priority: Some(Priority {
                    name: "High".to_string(),
                }),
                created: Some("2024-01-15T10:00:00.000+0000".to_string()),
                updated: Some("2024-01-16T14:30:00.000+0000".to_string()),
                description: None,
            },
        }
    }

    #[test]
    fn format_search_result_shows_issue_count_and_details() {
        // Given: a search result with multiple issues
        let result = SearchResult {
            total: 2,
            max_results: 50,
            start_at: 0,
            issues: vec![
                create_test_issue("PROJ-1", "First issue", "Open", "Alice"),
                create_test_issue("PROJ-2", "Second issue", "In Progress", "Bob"),
            ],
        };

        // When: formatting the result
        let output = format_search_result(&result);

        // Then: the output contains issue count and details
        assert!(output.contains("Found 2 issues"));
        assert!(output.contains("PROJ-1"));
        assert!(output.contains("First issue"));
        assert!(output.contains("[Open]"));
        assert!(output.contains("Alice"));
        assert!(output.contains("PROJ-2"));
        assert!(output.contains("Second issue"));
        assert!(output.contains("[In Progress]"));
        assert!(output.contains("Bob"));
    }

    #[test]
    fn format_search_result_handles_empty_results() {
        // Given: an empty search result
        let result = SearchResult {
            total: 0,
            max_results: 50,
            start_at: 0,
            issues: vec![],
        };

        // When: formatting the result
        let output = format_search_result(&result);

        // Then: the output shows zero issues
        assert!(output.contains("Found 0 issues"));
        assert!(output.contains("showing 0 of 0"));
    }

    #[test]
    fn format_search_result_handles_missing_fields() {
        // Given: an issue with missing optional fields
        let issue = Issue {
            id: "10001".to_string(),
            key: "PROJ-1".to_string(),
            self_url: "https://example.atlassian.net/rest/api/3/issue/PROJ-1".to_string(),
            fields: IssueFields {
                summary: None,
                status: None,
                assignee: None,
                priority: None,
                created: None,
                updated: None,
                description: None,
            },
        };
        let result = SearchResult {
            total: 1,
            max_results: 50,
            start_at: 0,
            issues: vec![issue],
        };

        // When: formatting the result
        let output = format_search_result(&result);

        // Then: default values are shown
        assert!(output.contains("PROJ-1"));
        assert!(output.contains("[Unknown]"));
        assert!(output.contains("No summary"));
        assert!(output.contains("Unassigned"));
    }

    #[test]
    fn format_issue_shows_all_details() {
        // Given: a complete issue
        let issue = create_test_issue("PROJ-123", "Important bug fix", "Done", "Developer");

        // When: formatting the issue
        let output = format_issue(&issue);

        // Then: all details are shown
        assert!(output.contains("# PROJ-123 - Important bug fix"));
        assert!(output.contains("**Status:** Done"));
        assert!(output.contains("**Assignee:** Developer"));
        assert!(output.contains("**Priority:** High"));
        assert!(output.contains("**Created:** 2024-01-15T10:00:00.000+0000"));
        assert!(output.contains("**Updated:** 2024-01-16T14:30:00.000+0000"));
        assert!(output.contains("**URL:** https://example.atlassian.net/rest/api/3/issue/PROJ-123"));
    }

    #[test]
    fn format_issue_handles_missing_fields() {
        // Given: an issue with missing optional fields
        let issue = Issue {
            id: "10001".to_string(),
            key: "PROJ-1".to_string(),
            self_url: "https://example.atlassian.net/rest/api/3/issue/PROJ-1".to_string(),
            fields: IssueFields {
                summary: None,
                status: None,
                assignee: None,
                priority: None,
                created: None,
                updated: None,
                description: None,
            },
        };

        // When: formatting the issue
        let output = format_issue(&issue);

        // Then: default values are shown
        assert!(output.contains("# PROJ-1 - No summary"));
        assert!(output.contains("**Status:** Unknown"));
        assert!(output.contains("**Assignee:** Unassigned"));
        assert!(output.contains("**Priority:** None"));
        assert!(output.contains("**Created:** Unknown"));
        assert!(output.contains("**Updated:** Unknown"));
    }

    #[test]
    fn format_comment_shows_success_message_with_details() {
        // Given: a comment with complete information
        let comment = jira::Comment {
            id: "10100".to_string(),
            self_url: "https://example.atlassian.net/rest/api/3/issue/PROJ-123/comment/10100"
                .to_string(),
            author: Some(User {
                display_name: "Developer".to_string(),
                email_address: Some("dev@example.com".to_string()),
            }),
            created: Some("2024-01-17T09:00:00.000+0000".to_string()),
        };

        // When: formatting the comment
        let output = format_comment("PROJ-123", &comment);

        // Then: the success message with details is shown
        assert!(output.contains("Comment added successfully to PROJ-123"));
        assert!(output.contains("**Comment ID:** 10100"));
        assert!(output.contains("**Author:** Developer"));
        assert!(output.contains("**Created:** 2024-01-17T09:00:00.000+0000"));
    }

    #[test]
    fn format_comment_handles_missing_fields() {
        // Given: a comment with missing optional fields
        let comment = jira::Comment {
            id: "10101".to_string(),
            self_url: "https://example.atlassian.net/rest/api/3/issue/PROJ-456/comment/10101"
                .to_string(),
            author: None,
            created: None,
        };

        // When: formatting the comment
        let output = format_comment("PROJ-456", &comment);

        // Then: default values are shown
        assert!(output.contains("Comment added successfully to PROJ-456"));
        assert!(output.contains("**Comment ID:** 10101"));
        assert!(output.contains("**Author:** Unknown"));
        assert!(output.contains("**Created:** Unknown"));
    }
}

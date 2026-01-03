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
}

#[tool_handler]
impl rmcp::ServerHandler for JiraServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Jira MCP Server - Search and retrieve Jira issues".into()),
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

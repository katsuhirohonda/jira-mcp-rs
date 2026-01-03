use std::sync::Arc;

use rmcp::{
    handler::server::router::tool::ToolRouter,
    handler::server::tool::Parameters,
    model::*,
    tool, tool_handler, tool_router,
    ErrorData as McpError,
};

use crate::jira::JiraClient;
use crate::tools::{
    format_comment, format_issue, format_search_result,
    AddCommentParams, GetIssueParams, SearchIssuesParams,
};

#[derive(Clone)]
pub struct JiraServer {
    jira: Arc<JiraClient>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl JiraServer {
    pub fn new(jira: JiraClient) -> Self {
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
        match self
            .jira
            .add_comment(&params.issue_key, &params.comment)
            .await
        {
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
            instructions: Some(
                "Jira MCP Server - Search, retrieve, and comment on Jira issues".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

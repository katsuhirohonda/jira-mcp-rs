use std::sync::Arc;

use rmcp::{
    handler::server::router::tool::ToolRouter,
    handler::server::tool::Parameters,
    model::*,
    tool, tool_handler, tool_router,
    ErrorData as McpError,
};

use crate::jira::{JiraClient, UpdateIssueRequest};
use crate::tools::{
    format_children, format_comment, format_comments, format_issue, format_search_result,
    format_update_result, AddCommentParams, GetChildrenParams, GetCommentsParams, GetIssueParams,
    SearchIssuesParams, UpdateIssueParams,
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

    #[tool(description = "Get child issues of a parent issue. Works for both epics (returns stories/tasks) and regular issues (returns subtasks).")]
    async fn get_children(
        &self,
        Parameters(params): Parameters<GetChildrenParams>,
    ) -> Result<CallToolResult, McpError> {
        let max_results = params.max_results.unwrap_or(50).min(100);

        match self.jira.get_children(&params.parent_key, max_results).await {
            Ok(result) => {
                let output = format_children(&params.parent_key, &result);
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get children: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Get comments on a Jira issue with pagination support. Returns comments with author, date, and content.")]
    async fn get_comments(
        &self,
        Parameters(params): Parameters<GetCommentsParams>,
    ) -> Result<CallToolResult, McpError> {
        let start_at = params.start_at.unwrap_or(0);
        let max_results = params.max_results.unwrap_or(50).min(100);

        match self
            .jira
            .get_comments(&params.issue_key, start_at, max_results)
            .await
        {
            Ok(response) => {
                let output = format_comments(&params.issue_key, &response);
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get comments: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Update a Jira issue's fields. Can update summary, description, due date, priority, assignee, parent (epic), and labels.")]
    async fn update_issue(
        &self,
        Parameters(params): Parameters<UpdateIssueParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut update = UpdateIssueRequest::new();
        let mut updated_fields = Vec::new();

        if let Some(summary) = &params.summary {
            update = update.summary(summary);
            updated_fields.push("summary");
        }
        if let Some(description) = &params.description {
            update = update.description(description);
            updated_fields.push("description");
        }
        if let Some(due_date) = &params.due_date {
            update = update.due_date(due_date);
            updated_fields.push("due_date");
        }
        if let Some(priority) = &params.priority {
            update = update.priority(priority);
            updated_fields.push("priority");
        }
        if let Some(assignee_id) = &params.assignee_account_id {
            update = update.assignee(assignee_id);
            updated_fields.push("assignee");
        }
        if let Some(parent_key) = &params.parent_key {
            update = update.parent(parent_key);
            updated_fields.push("parent");
        }
        if let Some(labels) = &params.labels {
            let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            update = update.labels(label_refs);
            updated_fields.push("labels");
        }

        if updated_fields.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "No fields provided to update. Please specify at least one field to update.",
            )]));
        }

        match self.jira.update_issue(&params.issue_key, update).await {
            Ok(()) => {
                let output = format_update_result(&params.issue_key, &updated_fields);
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to update issue: {}",
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

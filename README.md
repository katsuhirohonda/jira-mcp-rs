# jira-mcp-rs

Rust implementation of an MCP (Model Context Protocol) server for Jira integration.

## Features

- **search_issues**: Search for Jira issues using JQL (Jira Query Language)
- **get_issue**: Get detailed information about a specific Jira issue
- **add_comment**: Add a comment to a Jira issue
- **update_issue**: Update issue fields (summary, due date, priority, assignee, parent/epic, labels)

## Requirements

- Rust 2024 edition
- Jira Cloud account with API access

## Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `JIRA_BASE_URL` | Your Jira instance URL (e.g., `https://your-domain.atlassian.net`) | Yes |
| `JIRA_EMAIL` | Email address for Jira authentication | Yes |
| `JIRA_API_TOKEN` | Jira API token ([Generate here](https://id.atlassian.com/manage-profile/security/api-tokens)) | Yes |

## Build

```bash
cargo build --release
```

## Usage with Claude Desktop

Add to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "jira": {
      "command": "/path/to/jira-mcp-rs/target/release/jira-mcp-rs",
      "env": {
        "JIRA_BASE_URL": "https://your-domain.atlassian.net",
        "JIRA_EMAIL": "your-email@example.com",
        "JIRA_API_TOKEN": "your-api-token"
      }
    }
  }
}
```

## Available Tools

### search_issues

Search for Jira issues using JQL.

**Parameters:**
- `jql` (string, required): JQL query string (e.g., `project = PROJ AND status = Open`)
- `max_results` (number, optional): Maximum number of results (default: 50, max: 100)

### get_issue

Get detailed information about a specific issue.

**Parameters:**
- `issue_key` (string, required): The issue key (e.g., `PROJ-123`)

### add_comment

Add a comment to a Jira issue.

**Parameters:**
- `issue_key` (string, required): The issue key (e.g., `PROJ-123`)
- `comment` (string, required): The comment text to add to the issue

### update_issue

Update a Jira issue's fields.

**Parameters:**
- `issue_key` (string, required): The issue key (e.g., `PROJ-123`)
- `summary` (string, optional): New summary/title for the issue
- `due_date` (string, optional): Due date in YYYY-MM-DD format (e.g., `2025-01-31`)
- `priority` (string, optional): Priority name (e.g., `High`, `Medium`, `Low`)
- `assignee_account_id` (string, optional): Assignee's account ID
- `parent_key` (string, optional): Parent issue key for subtasks or epic (e.g., `EPIC-123`)
- `labels` (array of strings, optional): Labels to set on the issue

## Project Structure

```
src/
├── main.rs          # Entry point
├── server.rs        # MCP server with tool definitions
├── jira/
│   ├── mod.rs       # Jira API client
│   └── models.rs    # Data structures (Issue, Comment, etc.)
└── tools/
    ├── mod.rs       # Module exports
    ├── params.rs    # Tool parameter definitions
    └── formatters.rs # Output formatting functions
```

## License

MIT

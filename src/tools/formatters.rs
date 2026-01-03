use crate::jira::{Comment, Issue, SearchResult};

pub fn format_search_result(result: &SearchResult) -> String {
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

pub fn format_issue(issue: &Issue) -> String {
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

pub fn format_comment(issue_key: &str, comment: &Comment) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jira::{IssueFields, Priority, Status, User};

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
        let result = SearchResult {
            total: 2,
            max_results: 50,
            start_at: 0,
            issues: vec![
                create_test_issue("PROJ-1", "First issue", "Open", "Alice"),
                create_test_issue("PROJ-2", "Second issue", "In Progress", "Bob"),
            ],
        };

        let output = format_search_result(&result);

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
        let result = SearchResult {
            total: 0,
            max_results: 50,
            start_at: 0,
            issues: vec![],
        };

        let output = format_search_result(&result);

        assert!(output.contains("Found 0 issues"));
        assert!(output.contains("showing 0 of 0"));
    }

    #[test]
    fn format_search_result_handles_missing_fields() {
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

        let output = format_search_result(&result);

        assert!(output.contains("PROJ-1"));
        assert!(output.contains("[Unknown]"));
        assert!(output.contains("No summary"));
        assert!(output.contains("Unassigned"));
    }

    #[test]
    fn format_issue_shows_all_details() {
        let issue = create_test_issue("PROJ-123", "Important bug fix", "Done", "Developer");

        let output = format_issue(&issue);

        assert!(output.contains("# PROJ-123 - Important bug fix"));
        assert!(output.contains("**Status:** Done"));
        assert!(output.contains("**Assignee:** Developer"));
        assert!(output.contains("**Priority:** High"));
        assert!(output.contains("**Created:** 2024-01-15T10:00:00.000+0000"));
        assert!(output.contains("**Updated:** 2024-01-16T14:30:00.000+0000"));
        assert!(output.contains(
            "**URL:** https://example.atlassian.net/rest/api/3/issue/PROJ-123"
        ));
    }

    #[test]
    fn format_issue_handles_missing_fields() {
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

        let output = format_issue(&issue);

        assert!(output.contains("# PROJ-1 - No summary"));
        assert!(output.contains("**Status:** Unknown"));
        assert!(output.contains("**Assignee:** Unassigned"));
        assert!(output.contains("**Priority:** None"));
        assert!(output.contains("**Created:** Unknown"));
        assert!(output.contains("**Updated:** Unknown"));
    }

    #[test]
    fn format_comment_shows_success_message_with_details() {
        let comment = Comment {
            id: "10100".to_string(),
            self_url: "https://example.atlassian.net/rest/api/3/issue/PROJ-123/comment/10100"
                .to_string(),
            author: Some(User {
                display_name: "Developer".to_string(),
                email_address: Some("dev@example.com".to_string()),
            }),
            created: Some("2024-01-17T09:00:00.000+0000".to_string()),
        };

        let output = format_comment("PROJ-123", &comment);

        assert!(output.contains("Comment added successfully to PROJ-123"));
        assert!(output.contains("**Comment ID:** 10100"));
        assert!(output.contains("**Author:** Developer"));
        assert!(output.contains("**Created:** 2024-01-17T09:00:00.000+0000"));
    }

    #[test]
    fn format_comment_handles_missing_fields() {
        let comment = Comment {
            id: "10101".to_string(),
            self_url: "https://example.atlassian.net/rest/api/3/issue/PROJ-456/comment/10101"
                .to_string(),
            author: None,
            created: None,
        };

        let output = format_comment("PROJ-456", &comment);

        assert!(output.contains("Comment added successfully to PROJ-456"));
        assert!(output.contains("**Comment ID:** 10101"));
        assert!(output.contains("**Author:** Unknown"));
        assert!(output.contains("**Created:** Unknown"));
    }
}

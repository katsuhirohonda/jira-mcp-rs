use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Convert Markdown text to Atlassian Document Format (ADF) JSON
fn markdown_to_adf(markdown: &str) -> serde_json::Value {
    let parser = Parser::new_ext(markdown, Options::all());

    let mut doc_content: Vec<serde_json::Value> = Vec::new();
    let mut current_paragraph: Vec<serde_json::Value> = Vec::new();
    let mut current_list_items: Vec<serde_json::Value> = Vec::new();
    let mut current_list_item_content: Vec<serde_json::Value> = Vec::new();
    let mut in_list = false;
    let mut in_list_item = false;
    let mut bold = false;
    let mut italic = false;
    let mut code = false;
    let mut heading_level: Option<u32> = None;
    let mut current_heading: Vec<serde_json::Value> = Vec::new();

    let flush_paragraph =
        |content: &mut Vec<serde_json::Value>, doc: &mut Vec<serde_json::Value>| {
            if !content.is_empty() {
                doc.push(serde_json::json!({
                    "type": "paragraph",
                    "content": content.clone()
                }));
                content.clear();
            }
        };

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_paragraph(&mut current_paragraph, &mut doc_content);
                heading_level = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                });
                current_heading.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(level) = heading_level.take() {
                    doc_content.push(serde_json::json!({
                        "type": "heading",
                        "attrs": { "level": level },
                        "content": current_heading.clone()
                    }));
                    current_heading.clear();
                }
            }
            Event::Start(Tag::Paragraph) => {
                if !in_list {
                    current_paragraph.clear();
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if !in_list {
                    flush_paragraph(&mut current_paragraph, &mut doc_content);
                }
            }
            Event::Start(Tag::List(_)) => {
                flush_paragraph(&mut current_paragraph, &mut doc_content);
                in_list = true;
                current_list_items.clear();
            }
            Event::End(TagEnd::List(_)) => {
                if !current_list_item_content.is_empty() {
                    current_list_items.push(serde_json::json!({
                        "type": "listItem",
                        "content": [{
                            "type": "paragraph",
                            "content": current_list_item_content.clone()
                        }]
                    }));
                    current_list_item_content.clear();
                }
                doc_content.push(serde_json::json!({
                    "type": "bulletList",
                    "content": current_list_items.clone()
                }));
                current_list_items.clear();
                in_list = false;
            }
            Event::Start(Tag::Item) => {
                if !current_list_item_content.is_empty() {
                    current_list_items.push(serde_json::json!({
                        "type": "listItem",
                        "content": [{
                            "type": "paragraph",
                            "content": current_list_item_content.clone()
                        }]
                    }));
                    current_list_item_content.clear();
                }
                in_list_item = true;
            }
            Event::End(TagEnd::Item) => {
                in_list_item = false;
            }
            Event::Start(Tag::Strong) => bold = true,
            Event::End(TagEnd::Strong) => bold = false,
            Event::Start(Tag::Emphasis) => italic = true,
            Event::End(TagEnd::Emphasis) => italic = false,
            Event::Start(Tag::CodeBlock(_)) => code = true,
            Event::End(TagEnd::CodeBlock) => code = false,
            Event::Code(text) => {
                let node = serde_json::json!({
                    "type": "text",
                    "text": text.as_ref(),
                    "marks": [{ "type": "code" }]
                });
                if heading_level.is_some() {
                    current_heading.push(node);
                } else if in_list_item {
                    current_list_item_content.push(node);
                } else {
                    current_paragraph.push(node);
                }
            }
            Event::Text(text) => {
                if code {
                    doc_content.push(serde_json::json!({
                        "type": "codeBlock",
                        "content": [{ "type": "text", "text": text.as_ref() }]
                    }));
                    continue;
                }
                let mut marks: Vec<serde_json::Value> = Vec::new();
                if bold {
                    marks.push(serde_json::json!({ "type": "strong" }));
                }
                if italic {
                    marks.push(serde_json::json!({ "type": "em" }));
                }
                let node = if marks.is_empty() {
                    serde_json::json!({ "type": "text", "text": text.as_ref() })
                } else {
                    serde_json::json!({ "type": "text", "text": text.as_ref(), "marks": marks })
                };
                if heading_level.is_some() {
                    current_heading.push(node);
                } else if in_list_item {
                    current_list_item_content.push(node);
                } else {
                    current_paragraph.push(node);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                let node = serde_json::json!({ "type": "hardBreak" });
                if heading_level.is_some() {
                    current_heading.push(node);
                } else if in_list_item {
                    current_list_item_content.push(node);
                } else {
                    current_paragraph.push(node);
                }
            }
            _ => {}
        }
    }

    // flush remaining paragraph
    flush_paragraph(&mut current_paragraph, &mut doc_content);

    serde_json::json!({
        "type": "doc",
        "version": 1,
        "content": doc_content
    })
}

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
    pub total: Option<u32>,
    pub max_results: Option<u32>,
    pub start_at: Option<u32>,
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
    #[serde(rename = "issuetype")]
    pub issue_type: Option<IssueType>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub description: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IssueType {
    pub name: String,
    pub subtask: bool,
}

/// Response from GET /rest/api/2/issue/{issueIdOrKey}/comment
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentResponse {
    pub start_at: u32,
    pub max_results: u32,
    pub total: u32,
    pub comments: Vec<Comment>,
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
    #[serde(rename = "accountId")]
    pub account_id: Option<String>, // Account ID is optional as some users (like apps) might not have it in the same context, or for backward compatibility
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Priority {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AddCommentRequest {
    pub body: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Comment {
    pub id: String,
    #[serde(rename = "self")]
    pub self_url: String,
    pub author: Option<User>,
    pub created: Option<String>,
    pub body: Option<serde_json::Value>,
}

/// Request body for creating a new issue.
#[derive(Debug, Serialize, Default)]
pub struct CreateIssueRequest {
    pub fields: HashMap<String, serde_json::Value>,
}

impl CreateIssueRequest {
    pub fn new(project_key: &str, summary: &str, issue_type: &str) -> Self {
        let mut fields = HashMap::new();
        fields.insert(
            "project".to_string(),
            serde_json::json!({"key": project_key}),
        );
        fields.insert("summary".to_string(), serde_json::json!(summary));
        fields.insert(
            "issuetype".to_string(),
            serde_json::json!({"name": issue_type}),
        );
        Self { fields }
    }

    /// Set the description (Markdown converted to Atlassian Document Format)
    pub fn description(mut self, description: &str) -> Self {
        self.fields
            .insert("description".to_string(), markdown_to_adf(description));
        self
    }

    /// Set the priority by name (e.g., "High", "Medium", "Low")
    pub fn priority(mut self, priority_name: &str) -> Self {
        self.fields.insert(
            "priority".to_string(),
            serde_json::json!({"name": priority_name}),
        );
        self
    }

    /// Set the assignee by account ID
    pub fn assignee(mut self, account_id: &str) -> Self {
        self.fields.insert(
            "assignee".to_string(),
            serde_json::json!({"accountId": account_id}),
        );
        self
    }

    /// Set the parent issue (for subtasks or epic)
    pub fn parent(mut self, parent_key: &str) -> Self {
        self.fields.insert(
            "parent".to_string(),
            serde_json::json!({"key": parent_key}),
        );
        self
    }

    /// Set labels
    pub fn labels(mut self, labels: Vec<&str>) -> Self {
        self.fields
            .insert("labels".to_string(), serde_json::json!(labels));
        self
    }

    /// Set due date (format: "YYYY-MM-DD")
    pub fn due_date(mut self, date: &str) -> Self {
        self.fields
            .insert("duedate".to_string(), serde_json::json!(date));
        self
    }
}

/// Response from POST /rest/api/3/issue
#[derive(Debug, Deserialize, Serialize)]
pub struct CreatedIssue {
    pub id: String,
    pub key: String,
    #[serde(rename = "self")]
    pub self_url: String,
}

/// Request body for updating an issue.
/// Uses HashMap to allow flexible field updates.
#[derive(Debug, Serialize, Default)]
pub struct UpdateIssueRequest {
    /// Fields to update (e.g., "summary", "duedate", "priority", "assignee", "parent")
    pub fields: HashMap<String, serde_json::Value>,
}

impl UpdateIssueRequest {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the summary (title) of the issue
    pub fn summary(mut self, summary: &str) -> Self {
        self.fields
            .insert("summary".to_string(), serde_json::json!(summary));
        self
    }

    /// Set the due date (format: "YYYY-MM-DD")
    pub fn due_date(mut self, date: &str) -> Self {
        self.fields
            .insert("duedate".to_string(), serde_json::json!(date));
        self
    }

    /// Set the priority by name (e.g., "High", "Medium", "Low")
    pub fn priority(mut self, priority_name: &str) -> Self {
        self.fields.insert(
            "priority".to_string(),
            serde_json::json!({"name": priority_name}),
        );
        self
    }

    /// Set the assignee by account ID
    pub fn assignee(mut self, account_id: &str) -> Self {
        self.fields.insert(
            "assignee".to_string(),
            serde_json::json!({"accountId": account_id}),
        );
        self
    }

    /// Set the parent issue (for subtasks or moving to an epic)
    pub fn parent(mut self, parent_key: &str) -> Self {
        self.fields.insert(
            "parent".to_string(),
            serde_json::json!({"key": parent_key}),
        );
        self
    }

    /// Set labels
    pub fn labels(mut self, labels: Vec<&str>) -> Self {
        self.fields
            .insert("labels".to_string(), serde_json::json!(labels));
        self
    }

    /// Set the description (Markdown converted to Atlassian Document Format)
    pub fn description(mut self, description: &str) -> Self {
        self.fields
            .insert("description".to_string(), markdown_to_adf(description));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_content(adf: &serde_json::Value) -> &Vec<serde_json::Value> {
        adf["content"].as_array().unwrap()
    }

    #[test]
    fn converts_plain_text_to_paragraph() {
        let adf = markdown_to_adf("Hello world");
        let content = get_content(&adf);
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "paragraph");
        assert_eq!(content[0]["content"][0]["text"], "Hello world");
    }

    #[test]
    fn converts_heading_levels() {
        let adf = markdown_to_adf("## Overview\n\nsome text");
        let content = get_content(&adf);
        let heading = content.iter().find(|n| n["type"] == "heading").unwrap();
        assert_eq!(heading["attrs"]["level"], 2);
        assert_eq!(heading["content"][0]["text"], "Overview");
    }

    #[test]
    fn converts_bold_text() {
        let adf = markdown_to_adf("**bold** text");
        let content = get_content(&adf);
        let para = &content[0];
        let bold_node = &para["content"][0];
        assert_eq!(bold_node["text"], "bold");
        let marks = bold_node["marks"].as_array().unwrap();
        assert!(marks.iter().any(|m| m["type"] == "strong"));
    }

    #[test]
    fn converts_italic_text() {
        let adf = markdown_to_adf("*italic* text");
        let content = get_content(&adf);
        let italic_node = &content[0]["content"][0];
        assert_eq!(italic_node["text"], "italic");
        let marks = italic_node["marks"].as_array().unwrap();
        assert!(marks.iter().any(|m| m["type"] == "em"));
    }

    #[test]
    fn converts_bullet_list() {
        let adf = markdown_to_adf("- item one\n- item two\n- item three");
        let content = get_content(&adf);
        let list = content.iter().find(|n| n["type"] == "bulletList").unwrap();
        let items = list["content"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["type"], "listItem");
    }

    #[test]
    fn converts_inline_code() {
        let adf = markdown_to_adf("use `cargo build` to compile");
        let content = get_content(&adf);
        let nodes = content[0]["content"].as_array().unwrap();
        let code_node = nodes.iter().find(|n| {
            n["marks"]
                .as_array()
                .map(|m| m.iter().any(|mark| mark["type"] == "code"))
                .unwrap_or(false)
        });
        assert!(code_node.is_some());
        assert_eq!(code_node.unwrap()["text"], "cargo build");
    }

    #[test]
    fn converts_mixed_markdown() {
        let md = "## Title\n\nsome paragraph\n\n- item 1\n- item 2";
        let adf = markdown_to_adf(md);
        let content = get_content(&adf);
        assert!(content.iter().any(|n| n["type"] == "heading"));
        assert!(content.iter().any(|n| n["type"] == "paragraph"));
        assert!(content.iter().any(|n| n["type"] == "bulletList"));
    }

    #[test]
    fn wraps_adf_in_doc_node() {
        let adf = markdown_to_adf("test");
        assert_eq!(adf["type"], "doc");
        assert_eq!(adf["version"], 1);
    }
}

#[cfg(test)]
mod serialization_tests {
    use super::*;

    #[test]
    fn print_update_request_json() {
        let update = UpdateIssueRequest::new()
            .description("## 概要\nテスト内容\n\n- item 1\n- item 2");
        let json = serde_json::to_string_pretty(&update).unwrap();
        println!("{}", json);
    }
}

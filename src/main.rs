mod jira;
mod server;
mod tools;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};

use jira::JiraClient;
use server::JiraServer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let base_url =
        std::env::var("JIRA_BASE_URL").expect("JIRA_BASE_URL environment variable is required");
    let email = std::env::var("JIRA_EMAIL").expect("JIRA_EMAIL environment variable is required");
    let api_token =
        std::env::var("JIRA_API_TOKEN").expect("JIRA_API_TOKEN environment variable is required");

    let jira = JiraClient::new(&base_url, &email, &api_token);
    let server = JiraServer::new(jira);

    tracing::info!("Starting Jira MCP server...");

    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

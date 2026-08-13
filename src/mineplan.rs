use crate::analyzer::Edge;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
pub enum MineplanError {
    #[error("mineplan request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("mineplan returned an MCP error: {0}")]
    Mcp(String),
    #[error("mineplan returned an invalid focus response: {0}")]
    InvalidResponse(String),
}

#[derive(Clone)]
pub struct MineplanClient {
    client: Client,
    endpoint: String,
}

#[derive(Debug, Deserialize)]
struct FocusOutput {
    #[serde(default)]
    connections: Vec<Edge>,
}

impl MineplanClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.into(),
        }
    }

    pub async fn focus(&self, node: &str, limit: usize) -> Result<Vec<Edge>, MineplanError> {
        let response: Value = self
            .client
            .post(&self.endpoint)
            .json(&json!({
                "jsonrpc":"2.0",
                "id":1,
                "method":"tools/call",
                "params":{"name":"focus","arguments":{
                    "focus":node,
                    "limit":limit,
                    "include_connections":true
                }}
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if let Some(error) = response.get("error") {
            return Err(MineplanError::Mcp(
                error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
                    .into(),
            ));
        }
        let text = response
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .ok_or_else(|| MineplanError::InvalidResponse(response.to_string()))?;
        serde_json::from_str::<FocusOutput>(text)
            .map(|output| output.connections)
            .map_err(|error| MineplanError::InvalidResponse(error.to_string()))
    }
}

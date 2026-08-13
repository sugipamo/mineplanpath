use crate::analyzer::{Edge, find_path};
use crate::mineplan::MineplanClient;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashSet, VecDeque};

#[derive(Clone)]
pub struct Analyzer {
    mineplan: MineplanClient,
    max_focus_calls: usize,
    focus_limit: usize,
}

impl Analyzer {
    pub fn new(mineplan: MineplanClient, max_focus_calls: usize, focus_limit: usize) -> Self {
        Self {
            mineplan,
            max_focus_calls,
            focus_limit,
        }
    }

    pub async fn find_path(&self, from: &str, to: &str) -> Result<Value, String> {
        if from == to {
            return Ok(json!({"from":from,"to":to,"turns":0,"tasks":[]}));
        }
        let mut edges: BTreeMap<i64, Edge> = BTreeMap::new();
        let mut queue = VecDeque::from([from.to_string()]);
        let mut scheduled = HashSet::from([from.to_string()]);
        let mut focused = HashSet::new();

        while focused.len() < self.max_focus_calls {
            let Some(node) = queue.pop_front() else {
                break;
            };
            if !focused.insert(node.clone()) {
                continue;
            }
            let connections = self
                .mineplan
                .focus(&node, self.focus_limit)
                .await
                .map_err(|error| error.to_string())?;
            for edge in connections {
                for endpoint in [&edge.previous, &edge.next] {
                    if scheduled.insert(endpoint.clone()) {
                        queue.push_back(endpoint.clone());
                    }
                }
                edges.entry(edge.edge_id).or_insert(edge);
            }
            let observed: Vec<Edge> = edges.values().cloned().collect();
            if let Some(path) = find_path(&observed, from, to) {
                return Ok(json!({
                    "from":from,
                    "to":to,
                    "turns":path.turns,
                    "tasks":path.tasks
                }));
            }
        }
        Ok(json!({"from":from,"to":to,"found":false}))
    }
}

#[derive(Debug, Deserialize)]
struct Request {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

pub async fn handle_json_request(analyzer: &Analyzer, request: Value) -> Value {
    let request = match serde_json::from_value::<Request>(request) {
        Ok(request) => request,
        Err(error) => return rpc_error(Value::Null, -32600, format!("invalid request: {error}")),
    };
    let id = request.id.unwrap_or(Value::Null);
    let notification = id.is_null();
    let result = match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion":"2025-06-18",
            "capabilities":{"tools":{}},
            "serverInfo":{"name":"plan-path","version":env!("CARGO_PKG_VERSION")}
        })),
        "ping" => Ok(json!({})),
        "notifications/initialized" | "notifications/cancelled" => return Value::Null,
        "tools/list" => Ok(json!({"tools":[tool_definition()]})),
        "tools/call" => call_tool(analyzer, request.params).await,
        _ => Err((-32601, format!("method not found: {}", request.method))),
    };
    if notification {
        return Value::Null;
    }
    match result {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
        Err((code, message)) => rpc_error(id, code, message),
    }
}

async fn call_tool(analyzer: &Analyzer, params: Value) -> Result<Value, (i32, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "tools/call requires name".into()))?;
    if name != "find_path" {
        return Err((-32602, format!("unknown tool: {name}")));
    }
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(Map::new);
    let from = required_string(&arguments, "from")?;
    let to = required_string(&arguments, "to")?;
    let output = analyzer
        .find_path(from, to)
        .await
        .map_err(|message| (-32000, message))?;
    Ok(json!({
        "content":[{"type":"text","text":serde_json::to_string(&output).expect("serializable output")}]
    }))
}

fn tool_definition() -> Value {
    json!({
        "name":"find_path",
        "description":"Find one low-turn route between two mineplan nodes. A turn is a change of edge_name. The result is based on the graph observed through bounded focus calls, so found=false means no route was found in that observed range.",
        "inputSchema":{
            "type":"object",
            "properties":{"from":{"type":"string"},"to":{"type":"string"}},
            "required":["from","to"],
            "additionalProperties":false
        }
    })
}

fn required_string<'a>(
    arguments: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, (i32, String)> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or((-32602, format!("{name} must be a non-empty string")))
}

fn rpc_error(id: Value, code: i32, message: String) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn exposes_only_find_path() {
        let analyzer = Analyzer::new(MineplanClient::new("http://127.0.0.1:1/mcp"), 50, 50);
        let response = handle_json_request(
            &analyzer,
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        )
        .await;
        assert_eq!(response["result"]["tools"][0]["name"], "find_path");
        assert_eq!(response["result"]["tools"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn equal_endpoints_do_not_contact_mineplan() {
        let analyzer = Analyzer::new(MineplanClient::new("http://127.0.0.1:1/mcp"), 50, 50);
        let output = analyzer.find_path("A", "A").await.unwrap();
        assert_eq!(output, json!({"from":"A","to":"A","turns":0,"tasks":[]}));
    }
}

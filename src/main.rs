use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use plan_path::mcp::{self, Analyzer};
use plan_path::mineplan::MineplanClient;
use serde_json::Value;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    match env::args().nth(1).as_deref() {
        Some("help" | "--help" | "-h") => {
            println!("{}", help_text());
            return Ok(());
        }
        Some("version" | "--version" | "-V") => {
            println!("plan-path {}", env!("PLAN_PATH_BUILD_VERSION"));
            return Ok(());
        }
        Some(argument) => {
            return Err(format!("unknown command: {argument}\n\n{}", help_text()).into());
        }
        None => {}
    }
    let port = env_usize("PLAN_PATH_HTTP_PORT", 3100)?;
    let max_focus_calls = env_usize("PLAN_PATH_MAX_FOCUS_CALLS", 50)?;
    let focus_limit = env_usize("PLAN_PATH_FOCUS_LIMIT", 50)?;
    let mineplan_url =
        env::var("MINEPLAN_MCP_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/mcp".into());
    let bind = format!("127.0.0.1:{port}");
    let analyzer = Analyzer::new(
        MineplanClient::new(&mineplan_url),
        max_focus_calls,
        focus_limit,
    );
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    eprintln!("plan-path MCP: http://{bind}/mcp");
    eprintln!("mineplan MCP: {mineplan_url}");
    axum::serve(listener, app(analyzer)).await?;
    Ok(())
}

fn env_usize(name: &str, default: usize) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(env::var(name).map_or(Ok(default), |value| value.parse())?)
}

fn app(analyzer: Analyzer) -> Router {
    Router::new()
        .route("/mcp", post(mcp_post).get(mcp_get))
        .with_state(analyzer)
}

async fn mcp_post(
    State(analyzer): State<Analyzer>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Response {
    if headers.get(header::ORIGIN).is_some() {
        return StatusCode::FORBIDDEN.into_response();
    }
    Json(mcp::handle_json_request(&analyzer, request).await).into_response()
}

async fn mcp_get(headers: HeaderMap) -> Response {
    if headers.get(header::ORIGIN).is_some() {
        return StatusCode::FORBIDDEN.into_response();
    }
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}

fn help_text() -> &'static str {
    "plan-path MCP server

USAGE:
  plan-path
  plan-path help
  plan-path version

ENVIRONMENT:
  MINEPLAN_MCP_URL           mineplan endpoint (default: http://127.0.0.1:3000/mcp)
  PLAN_PATH_HTTP_PORT        local HTTP port (default: 3100)
  PLAN_PATH_MAX_FOCUS_CALLS  maximum focus calls per search (default: 50)
  PLAN_PATH_FOCUS_LIMIT      limit passed to each focus call (default: 50)

MCP:
  POST http://127.0.0.1:3100/mcp
  Tools: find_path"
}

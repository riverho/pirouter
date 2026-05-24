//! HTTP server — OpenAI-compatible surface.
//!
//! Exposes:
//!   POST /v1/chat/completions  — main proxy
//!   GET  /v1/models            — list of model aliases declared in config
//!   GET  /healthz              — liveness
//!
//! For v0 we accept `stream: true` but transparently buffer and re-emit a
//! single SSE chunk plus `[DONE]`. True streaming is a v0.x item that
//! requires streaming-aware cascade.

use crate::config::Config;
use crate::ledger::{Ledger, LedgerRow};
use crate::providers::Providers;
use crate::router::{cascade, policy};
use crate::types::{Message, Request as IRequest, Role, Tool};
use anyhow::Result;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{sse::Event, IntoResponse, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub providers: Arc<Providers>,
    pub ledger: Arc<Ledger>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

pub async fn serve(cfg: Config) -> Result<()> {
    let providers = Providers::from_config(&cfg)?;
    let ledger = Ledger::open(&cfg).await?;
    let bind = cfg.server.bind.clone();
    let state = AppState {
        cfg: Arc::new(cfg),
        providers: Arc::new(providers),
        ledger: Arc::new(ledger),
    };
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(bind = %bind, "pirouter listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}

#[derive(Serialize)]
struct ModelEntry {
    id: String,
    object: &'static str,
    owned_by: String,
}

async fn list_models(State(state): State<AppState>) -> Json<Value> {
    let data: Vec<ModelEntry> = state
        .cfg
        .models
        .iter()
        .map(|(alias, m)| ModelEntry {
            id: alias.clone(),
            object: "model",
            owned_by: m.provider.to_string(),
        })
        .collect();
    Json(json!({ "object": "list", "data": data }))
}

// -- OpenAI request shape ----------------------------------------------------

#[derive(Deserialize)]
struct OAIIncoming {
    model: String,
    messages: Vec<OAIIncomingMessage>,
    #[serde(default)]
    tools: Vec<Value>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    max_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct OAIIncomingMessage {
    role: String,
    content: Value,
    #[serde(default)]
    name: Option<String>,
}

fn message_text(content: &Value) -> String {
    // OpenAI permits either a string or an array of typed parts.
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(|t| t.as_str()).map(str::to_string))
            .collect::<Vec<_>>()
            .join(""),
        other => other.to_string(),
    }
}

fn role_from_str(s: &str) -> Role {
    match s {
        "system" => Role::System,
        "assistant" => Role::Assistant,
        "tool" => Role::Tool,
        _ => Role::User,
    }
}

fn tool_from_value(v: &Value) -> Option<Tool> {
    let fnc = v.get("function")?;
    Some(Tool {
        name: fnc.get("name")?.as_str()?.to_string(),
        description: fnc
            .get("description")
            .and_then(|d| d.as_str())
            .map(str::to_string),
        parameters: fnc
            .get("parameters")
            .cloned()
            .unwrap_or(Value::Object(Default::default())),
    })
}

fn headers_to_map(headers: &HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|s| (k.as_str().to_lowercase(), s.to_string()))
        })
        .collect()
}

async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<OAIIncoming>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let started = Instant::now();

    let internal = IRequest {
        requested_model: body.model.clone(),
        messages: body
            .messages
            .iter()
            .map(|m| Message {
                role: role_from_str(&m.role),
                content: message_text(&m.content),
                name: m.name.clone(),
                extra: HashMap::new(),
            })
            .collect(),
        tools: body.tools.iter().filter_map(tool_from_value).collect(),
        stream: body.stream,
        temperature: body.temperature,
        max_tokens: body.max_tokens,
        headers: headers_to_map(&headers),
    };

    let decision = policy::decide(&state.cfg, &internal)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let outcome = cascade::execute(&state.cfg, &state.providers, &decision, &internal)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let latency_ms = started.elapsed().as_millis() as u64;

    let Some(resp) = outcome.final_response.clone() else {
        let _ = state
            .ledger
            .record(
                &state.cfg,
                LedgerRow {
                    requested_model: &internal.requested_model,
                    rule: Some(&decision.rule_name),
                    primary_model: &decision.primary_alias,
                    final_model: &decision.primary_alias,
                    outcome: &outcome,
                    total_latency_ms: latency_ms,
                    status: "exhausted",
                },
            )
            .await;
        return Err((
            StatusCode::BAD_GATEWAY,
            "cascade exhausted without a successful response".into(),
        ));
    };

    // Best-effort ledger write — never fail the request because of it.
    let final_alias = outcome.final_alias.clone().unwrap_or_default();
    if let Err(e) = state
        .ledger
        .record(
            &state.cfg,
            LedgerRow {
                requested_model: &internal.requested_model,
                rule: Some(&decision.rule_name),
                primary_model: &decision.primary_alias,
                final_model: &final_alias,
                outcome: &outcome,
                total_latency_ms: latency_ms,
                status: "ok",
            },
        )
        .await
    {
        tracing::warn!(error = %e, "ledger write failed");
    }

    let mut message = json!({ "role": "assistant", "content": resp.content });
    if !resp.tool_calls.is_empty() {
        message["tool_calls"] = Value::Array(resp.tool_calls.clone());
        if message["content"]
            .as_str()
            .is_some_and(|content| content.is_empty())
        {
            message["content"] = Value::Null;
        }
    }

    let openai_body = json!({
        "id": format!("chatcmpl-{}", ulid::Ulid::new()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": final_alias,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": resp.finish_reason.unwrap_or_else(|| "stop".into())
        }],
        "usage": {
            "prompt_tokens": resp.input_tokens,
            "completion_tokens": resp.output_tokens,
            "total_tokens": resp.input_tokens + resp.output_tokens
        },
        "pirouter": {
            "rule": decision.rule_name,
            "attempts": outcome.attempts,
        }
    });

    if internal.stream {
        Ok(stream_single_chunk(openai_body).into_response())
    } else {
        Ok(Json(openai_body).into_response())
    }
}

/// Emit the response as a single SSE `chat.completion.chunk` event plus
/// `[DONE]`. This keeps streaming clients happy until v0.x ships
/// real streaming.
fn stream_single_chunk(
    body: Value,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let message = body
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"));
    let finish_reason = body
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("finish_reason"))
        .and_then(|s| s.as_str())
        .unwrap_or("stop")
        .to_string();
    let model = body
        .get("model")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let id = body
        .get("id")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    let mut delta = json!({ "role": "assistant" });
    if let Some(content) = message
        .and_then(|m| m.get("content"))
        .and_then(|s| s.as_str())
    {
        delta["content"] = Value::String(content.to_string());
    }
    if let Some(tool_calls) = message.and_then(|m| m.get("tool_calls")).cloned() {
        delta["tool_calls"] = tool_calls;
    } else if delta.get("content").is_none() {
        delta["content"] = Value::String(String::new());
    }

    let chunk = json!({
        "id": id,
        "object": "chat.completion.chunk",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "delta": delta,
            "finish_reason": finish_reason
        }]
    });

    let events = vec![
        Ok(Event::default().data(chunk.to_string())),
        Ok(Event::default().data("[DONE]")),
    ];
    Sse::new(stream::iter(events))
}

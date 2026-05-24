//! Anthropic Messages API adapter.
//!
//! Translates internal `Request` -> Anthropic Messages API payload, and
//! the response back. The Anthropic API distinguishes a top-level `system`
//! string from the user/assistant message history; we hoist system
//! messages here.

use super::{http_client, provider_http_error, resolve_key, Provider};
use crate::config::ProviderCreds;
use crate::types::{Request, Response, Role};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub struct AnthropicProvider {
    creds: ProviderCreds,
    http: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(creds: &ProviderCreds) -> Result<Self> {
        Ok(Self {
            creds: creds.clone(),
            http: http_client(creds.request_timeout_secs),
        })
    }
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<AnthropicTool<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct AnthropicTool<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    input_schema: &'a Value,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<Value>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
    model: String,
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    async fn complete(&self, model_id: &str, req: &Request) -> Result<Response> {
        let api_key = resolve_key(&self.creds)?;
        let system = {
            let s = req.system_text();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        };

        let messages: Vec<AnthropicMessage> = req
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| AnthropicMessage {
                role: match m.role {
                    Role::User | Role::Tool => "user",
                    Role::Assistant => "assistant",
                    Role::System => unreachable!(),
                },
                content: m.content.as_str(),
            })
            .collect();

        let body = AnthropicRequest {
            model: model_id,
            messages,
            tools: req
                .tools
                .iter()
                .map(|tool| AnthropicTool {
                    name: &tool.name,
                    description: tool.description.as_deref(),
                    input_schema: &tool.parameters,
                })
                .collect(),
            system,
            max_tokens: req.max_tokens.unwrap_or(1024),
            temperature: req.temperature,
        };

        let url = format!("{}/v1/messages", self.creds.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("anthropic request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(provider_http_error("anthropic", status, text).into());
        }
        let parsed: AnthropicResponse = resp.json().await.context("anthropic decode")?;
        let content = parsed
            .content
            .iter()
            .filter(|b| b.block_type == "text")
            .filter_map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join("");
        let tool_calls = parsed
            .content
            .iter()
            .filter(|b| b.block_type == "tool_use")
            .filter_map(|b| {
                let id = b.id.as_ref()?;
                let name = b.name.as_ref()?;
                let input = b.input.as_ref().cloned().unwrap_or(Value::Null);
                Some(json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": serde_json::to_string(&input).unwrap_or_else(|_| "{}".into()),
                    }
                }))
            })
            .collect();
        let (input_tokens, output_tokens) = parsed
            .usage
            .map(|u| (u.input_tokens, u.output_tokens))
            .unwrap_or((0, 0));
        Ok(Response {
            model: parsed.model,
            content,
            finish_reason: parsed.stop_reason,
            input_tokens,
            output_tokens,
            tool_calls,
        })
    }
}

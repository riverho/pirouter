//! Ollama chat adapter.
//!
//! This is the first local-provider path. Ollama's native `/api/chat`
//! endpoint is close enough to the internal shape that we can keep the
//! adapter small while still supporting local-first routing profiles.

use super::{http_client, provider_http_error, Provider};
use crate::config::ProviderCreds;
use crate::types::{Request, Response, Role};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub struct OllamaProvider {
    base_url: String,
    http: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(creds: &ProviderCreds) -> Result<Self> {
        Ok(Self {
            base_url: creds.base_url.clone(),
            http: http_client(creds.request_timeout_secs),
        })
    }
}

#[derive(Serialize)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    model: String,
    message: OllamaRespMessage,
    #[serde(default)]
    prompt_eval_count: u32,
    #[serde(default)]
    eval_count: u32,
    #[serde(default)]
    done_reason: Option<String>,
}

#[derive(Deserialize)]
struct OllamaRespMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Vec<Value>,
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &'static str {
        "ollama"
    }

    async fn complete(&self, model_id: &str, req: &Request) -> Result<Response> {
        let messages: Vec<OllamaMessage> = req
            .messages
            .iter()
            .map(|m| OllamaMessage {
                role: match m.role {
                    Role::System => "system",
                    Role::User | Role::Tool => "user",
                    Role::Assistant => "assistant",
                },
                content: m.content.as_str(),
            })
            .collect();

        let options = if req.temperature.is_some() || req.max_tokens.is_some() {
            Some(OllamaOptions {
                temperature: req.temperature,
                num_predict: req.max_tokens,
            })
        } else {
            None
        };

        let body = OllamaRequest {
            model: model_id,
            messages,
            stream: false,
            tools: req
                .tools
                .iter()
                .map(|tool| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": &tool.name,
                            "description": &tool.description,
                            "parameters": &tool.parameters,
                        }
                    })
                })
                .collect(),
            options,
        };

        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("ollama request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(provider_http_error("ollama", status, text).into());
        }

        let parsed: OllamaResponse = resp.json().await.context("ollama decode")?;
        Ok(Response {
            model: parsed.model,
            content: parsed.message.content,
            finish_reason: parsed.done_reason,
            input_tokens: parsed.prompt_eval_count,
            output_tokens: parsed.eval_count,
            tool_calls: parsed.message.tool_calls,
        })
    }
}

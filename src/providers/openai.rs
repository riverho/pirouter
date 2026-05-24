//! OpenAI Chat Completions adapter.
//!
//! OpenAI's wire shape is already what we expose to clients, so this is
//! the lightest adapter. The main work is translating internal `Message`
//! enums back into the role strings OpenAI expects.

use super::{http_client, provider_http_error, resolve_key, Provider};
use crate::config::ProviderCreds;
use crate::types::{Request, Response, Role};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub struct OpenAiProvider {
    creds: ProviderCreds,
    http: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(creds: &ProviderCreds) -> Result<Self> {
        Ok(Self {
            creds: creds.clone(),
            http: http_client(creds.request_timeout_secs),
        })
    }
}

#[derive(Serialize)]
struct OAIMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct OAIRequest<'a> {
    model: &'a str,
    messages: Vec<OAIMessage<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct OAIChoice {
    message: OAIRespMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OAIRespMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<Value>,
}

#[derive(Deserialize)]
struct OAIUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct OAIResponse {
    model: String,
    choices: Vec<OAIChoice>,
    #[serde(default)]
    usage: Option<OAIUsage>,
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    async fn complete(&self, model_id: &str, req: &Request) -> Result<Response> {
        let api_key = resolve_key(&self.creds)?;
        let messages: Vec<OAIMessage> = req
            .messages
            .iter()
            .map(|m| OAIMessage {
                role: match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                },
                content: m.content.as_str(),
            })
            .collect();

        let body = OAIRequest {
            model: model_id,
            messages,
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
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        };

        let url = format!(
            "{}/v1/chat/completions",
            self.creds.base_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .context("openai request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(provider_http_error("openai", status, text).into());
        }
        let parsed: OAIResponse = resp.json().await.context("openai decode")?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("openai returned no choices"))?;
        let content = choice.message.content.unwrap_or_default();
        let (input_tokens, output_tokens) = parsed
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));
        Ok(Response {
            model: parsed.model,
            content,
            finish_reason: choice.finish_reason,
            input_tokens,
            output_tokens,
            tool_calls: choice.message.tool_calls,
        })
    }
}

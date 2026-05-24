//! Provider trait + dispatch.
//!
//! A `Provider` is anything that can take an internal `Request` plus a
//! provider-specific model ID and produce a `Response` (for v0 we keep it
//! non-streaming on the trait; the server still streams to the client by
//! re-chunking the final body — streaming-aware cascade is a v0.x item).

use crate::config::{Config, ProviderCreds, ProviderKind};
use crate::types::{Request, Response};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::StatusCode;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

pub mod anthropic;
pub mod ollama;
pub mod openai;

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, model_id: &str, req: &Request) -> Result<Response>;
    fn name(&self) -> &'static str;
}

/// Lookup table built once at startup. Each provider gets one shared client.
pub struct Providers {
    pub anthropic: Option<Arc<anthropic::AnthropicProvider>>,
    pub ollama: Option<Arc<ollama::OllamaProvider>>,
    pub openai: Option<Arc<openai::OpenAiProvider>>,
}

impl Providers {
    pub fn from_config(cfg: &Config) -> Result<Self> {
        let anthropic = cfg
            .providers
            .anthropic
            .as_ref()
            .map(|c| anthropic::AnthropicProvider::new(c).map(Arc::new))
            .transpose()?;
        let ollama = cfg
            .providers
            .ollama
            .as_ref()
            .map(|c| ollama::OllamaProvider::new(c).map(Arc::new))
            .transpose()?;
        let openai = cfg
            .providers
            .openai
            .as_ref()
            .map(|c| openai::OpenAiProvider::new(c).map(Arc::new))
            .transpose()?;
        Ok(Self {
            anthropic,
            ollama,
            openai,
        })
    }

    /// Resolve a provider by name. Returns an error if the named provider
    /// isn't configured (i.e. the user declared a model that points at a
    /// provider they haven't set credentials for).
    pub fn get(&self, kind: ProviderKind) -> Result<Arc<dyn Provider>> {
        match kind {
            ProviderKind::Anthropic => self
                .anthropic
                .clone()
                .map(|p| p as Arc<dyn Provider>)
                .ok_or_else(|| anyhow!("provider `anthropic` not configured")),
            ProviderKind::Openai => self
                .openai
                .clone()
                .map(|p| p as Arc<dyn Provider>)
                .ok_or_else(|| anyhow!("provider `openai` not configured")),
            ProviderKind::Ollama => self
                .ollama
                .clone()
                .map(|p| p as Arc<dyn Provider>)
                .ok_or_else(|| anyhow!("provider `ollama` not configured")),
        }
    }
}

/// Shared HTTP client construction — keeps connection pooling consistent
/// across providers.
pub(crate) fn http_client(request_timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("pirouter/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(request_timeout_secs.max(1)))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .build()
        .expect("reqwest client")
}

pub(crate) fn resolve_key(creds: &ProviderCreds) -> Result<String> {
    creds.resolved_key()
}

#[derive(Debug)]
pub struct ProviderHttpError {
    provider: &'static str,
    status: StatusCode,
    body: String,
}

impl ProviderHttpError {
    fn retryable(&self) -> bool {
        self.status == StatusCode::REQUEST_TIMEOUT
            || self.status == StatusCode::TOO_MANY_REQUESTS
            || self.status == StatusCode::PAYLOAD_TOO_LARGE
            || self.status.is_server_error()
            || is_context_error(self.status, &self.body)
    }
}

impl fmt::Display for ProviderHttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.provider, self.status, self.body)
    }
}

impl std::error::Error for ProviderHttpError {}

pub(crate) fn provider_http_error(
    provider: &'static str,
    status: StatusCode,
    body: String,
) -> ProviderHttpError {
    ProviderHttpError {
        provider,
        status,
        body,
    }
}

pub(crate) fn should_cascade_error(error: &anyhow::Error) -> bool {
    for cause in error.chain() {
        if let Some(provider_error) = cause.downcast_ref::<ProviderHttpError>() {
            return provider_error.retryable();
        }
        if let Some(reqwest_error) = cause.downcast_ref::<reqwest::Error>() {
            return reqwest_error.is_timeout() || reqwest_error.is_connect();
        }
    }
    false
}

fn is_context_error(status: StatusCode, body: &str) -> bool {
    if status != StatusCode::BAD_REQUEST && status != StatusCode::UNPROCESSABLE_ENTITY {
        return false;
    }
    let body = body.to_lowercase();
    body.contains("context") || body.contains("too many tokens") || body.contains("maximum token")
}

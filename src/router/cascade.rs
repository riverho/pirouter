//! Cascade execution.
//!
//! Given a `ResolvedDecision`, calls the primary model. On configured
//! escalation signals, tries the next model in the cascade. Each attempt
//! is recorded so the ledger can later show exactly which models were
//! touched and why.

use super::rules::ResolvedDecision;
use crate::config::{CascadeConfig, Config};
use crate::providers::{should_cascade_error, Providers};
use crate::types::{Request, Response};
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct CascadeAttempt {
    pub alias: String,
    pub provider: String,
    pub model_id: String,
    pub latency_ms: u64,
    pub outcome: AttemptOutcome,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AttemptOutcome {
    Ok {
        input_tokens: u32,
        output_tokens: u32,
    },
    EscalatedHttpError {
        message: String,
    },
    EscalatedShortResponse {
        input_tokens: u32,
        output_tokens: u32,
    },
    EscalatedMarker {
        input_tokens: u32,
        output_tokens: u32,
    },
}

/// The full result of a cascade run. `final_response` is set if some
/// attempt succeeded; otherwise the cascade exhausted its options.
#[derive(Debug)]
pub struct CascadeOutcome {
    pub attempts: Vec<CascadeAttempt>,
    pub final_response: Option<Response>,
    pub final_alias: Option<String>,
}

pub async fn execute(
    cfg: &Config,
    providers: &Providers,
    decision: &ResolvedDecision,
    req: &Request,
) -> Result<CascadeOutcome> {
    let chain: Vec<&String> = std::iter::once(&decision.primary_alias)
        .chain(decision.cascade_aliases.iter())
        .collect();

    let mut attempts = Vec::with_capacity(chain.len());

    for alias in chain {
        let model = cfg
            .models
            .get(alias)
            .ok_or_else(|| anyhow!("model alias `{alias}` not declared in config"))?;
        let provider = providers.get(model.provider)?;
        let started = Instant::now();
        let result = provider.complete(&model.model_id, req).await;
        let latency_ms = started.elapsed().as_millis() as u64;

        match result {
            Err(e) if cfg.cascade.on_http_error && should_cascade_error(&e) => {
                attempts.push(CascadeAttempt {
                    alias: alias.clone(),
                    provider: model.provider.to_string(),
                    model_id: model.model_id.clone(),
                    latency_ms,
                    outcome: AttemptOutcome::EscalatedHttpError {
                        message: e.to_string(),
                    },
                });
                continue;
            }
            Err(e) => return Err(e),
            Ok(resp) => {
                if should_escalate(&resp, &cfg.cascade) {
                    attempts.push(escalation_attempt(
                        alias,
                        model,
                        latency_ms,
                        &resp,
                        &cfg.cascade,
                    ));
                    continue;
                }
                attempts.push(CascadeAttempt {
                    alias: alias.clone(),
                    provider: model.provider.to_string(),
                    model_id: model.model_id.clone(),
                    latency_ms,
                    outcome: AttemptOutcome::Ok {
                        input_tokens: resp.input_tokens,
                        output_tokens: resp.output_tokens,
                    },
                });
                return Ok(CascadeOutcome {
                    attempts,
                    final_response: Some(resp),
                    final_alias: Some(alias.clone()),
                });
            }
        }
    }

    Ok(CascadeOutcome {
        attempts,
        final_response: None,
        final_alias: None,
    })
}

fn should_escalate(resp: &Response, cfg: &CascadeConfig) -> bool {
    if cfg.on_short_response && resp.output_tokens < cfg.min_output_tokens {
        return true;
    }
    if let Some(marker) = &cfg.on_marker {
        if resp.content.contains(marker) {
            return true;
        }
    }
    false
}

fn escalation_attempt(
    alias: &str,
    model: &crate::config::ModelConfig,
    latency_ms: u64,
    resp: &Response,
    cfg: &CascadeConfig,
) -> CascadeAttempt {
    let outcome = if cfg
        .on_marker
        .as_deref()
        .is_some_and(|m| resp.content.contains(m))
    {
        AttemptOutcome::EscalatedMarker {
            input_tokens: resp.input_tokens,
            output_tokens: resp.output_tokens,
        }
    } else {
        AttemptOutcome::EscalatedShortResponse {
            input_tokens: resp.input_tokens,
            output_tokens: resp.output_tokens,
        }
    };
    CascadeAttempt {
        alias: alias.to_string(),
        provider: model.provider.to_string(),
        model_id: model.model_id.clone(),
        latency_ms,
        outcome,
    }
}

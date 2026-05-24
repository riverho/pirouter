//! Capability-aware routing policy.
//!
//! Hand-written rules remain useful for explicit overrides, but the daemon
//! should also be able to make a good default choice from a model catalog.
//! This module provides that built-in policy router.

use super::rules::{self, ResolvedDecision};
use crate::config::{Config, ModelConfig, ModelQuality, RoutingProfile};
use crate::types::{Request, Role};
use anyhow::{anyhow, Result};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Difficulty {
    Easy,
    Standard,
    Hard,
}

pub fn decide(cfg: &Config, req: &Request) -> Result<ResolvedDecision> {
    if !cfg.rules.is_empty() {
        match rules::decide(cfg, req) {
            Ok(decision) => return validate_decision(cfg, decision),
            Err(e) if e.to_string().starts_with("no rule matched") => {}
            Err(e) => return Err(e),
        }
    }

    decide_by_policy(cfg, req)
}

fn validate_decision(cfg: &Config, decision: ResolvedDecision) -> Result<ResolvedDecision> {
    for alias in std::iter::once(&decision.primary_alias).chain(decision.cascade_aliases.iter()) {
        if !cfg.models.contains_key(alias) {
            return Err(anyhow!(
                "rule `{}` selected unknown model alias `{}`",
                decision.rule_name,
                alias
            ));
        }
    }
    Ok(decision)
}

fn decide_by_policy(cfg: &Config, req: &Request) -> Result<ResolvedDecision> {
    let difficulty = estimate_difficulty(req);
    let min_quality = required_quality(difficulty);
    let estimated_tokens = req.estimate_input_tokens();
    let needs_tools = req.has_tools();

    let mut candidates: Vec<(&String, &ModelConfig)> = cfg
        .models
        .iter()
        .filter(|(_, model)| model.enabled)
        .filter(|(_, model)| match cfg.routing.profile {
            RoutingProfile::CloudOnly => !model.local,
            _ => true,
        })
        .filter(|(_, model)| !needs_tools || model.supports_tools)
        .filter(|(_, model)| {
            model
                .context_window
                .map(|window| window >= estimated_tokens)
                .unwrap_or(true)
        })
        .collect();

    if candidates.is_empty() {
        return Err(anyhow!(
            "no enabled model satisfies routing constraints: profile={:?}, tools={}, estimated_tokens={}",
            cfg.routing.profile,
            needs_tools,
            estimated_tokens
        ));
    }

    let adequate = candidates
        .iter()
        .any(|(_, model)| model.quality >= min_quality);
    if adequate {
        candidates.retain(|(_, model)| model.quality >= min_quality);
    }

    candidates.sort_by(|a, b| primary_order(cfg.routing.profile, min_quality, a, b));
    let Some((primary_alias, primary_model)) = candidates.first().copied() else {
        return Err(anyhow!("no routeable models declared"));
    };

    let cascade_aliases = if cfg.routing.auto_cascade {
        cascade_order(primary_model, &candidates)
            .into_iter()
            .filter(|(alias, _)| *alias != primary_alias)
            .take(cfg.routing.max_policy_fallbacks)
            .map(|(alias, _)| alias.clone())
            .collect()
    } else {
        Vec::new()
    };

    Ok(ResolvedDecision {
        rule_name: format!(
            "policy:{}:{}",
            profile_name(cfg.routing.profile),
            difficulty_name(difficulty)
        ),
        primary_alias: primary_alias.clone(),
        cascade_aliases,
    })
}

fn estimate_difficulty(req: &Request) -> Difficulty {
    if let Some(hint) = req.headers.get("x-pirouter-difficulty") {
        match hint.to_lowercase().as_str() {
            "easy" | "basic" => return Difficulty::Easy,
            "standard" | "normal" | "medium" => return Difficulty::Standard,
            "hard" | "strong" | "premium" => return Difficulty::Hard,
            _ => {}
        }
    }

    let tokens = req.estimate_input_tokens();
    if req.has_tools() || tokens > 32_000 {
        return Difficulty::Hard;
    }
    if tokens > 8_000 {
        return Difficulty::Standard;
    }

    let text = req
        .messages
        .iter()
        .filter(|m| m.role == Role::System || m.role == Role::User)
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    let hard_markers = [
        "debug",
        "refactor",
        "architecture",
        "proof",
        "optimize",
        "security",
        "race condition",
        "distributed",
        "schema migration",
    ];
    if hard_markers.iter().any(|marker| text.contains(marker)) {
        return Difficulty::Hard;
    }

    let standard_markers = [
        "code",
        "sql",
        "test",
        "plan",
        "analyze",
        "compare",
        "summarize",
        "translate",
    ];
    if standard_markers.iter().any(|marker| text.contains(marker)) {
        return Difficulty::Standard;
    }

    Difficulty::Easy
}

fn required_quality(difficulty: Difficulty) -> ModelQuality {
    match difficulty {
        Difficulty::Easy => ModelQuality::Basic,
        Difficulty::Standard => ModelQuality::Standard,
        Difficulty::Hard => ModelQuality::Strong,
    }
}

fn primary_order(
    profile: RoutingProfile,
    min_quality: ModelQuality,
    a: &(&String, &ModelConfig),
    b: &(&String, &ModelConfig),
) -> Ordering {
    match profile {
        RoutingProfile::LocalFirst => {
            a.1.local
                .cmp(&b.1.local)
                .reverse()
                .then_with(|| {
                    quality_distance(a.1.quality, min_quality)
                        .cmp(&quality_distance(b.1.quality, min_quality))
                })
                .then_with(|| estimated_cost(a.1).total_cmp(&estimated_cost(b.1)))
                .then_with(|| a.0.cmp(b.0))
        }
        RoutingProfile::BestQuality => {
            b.1.quality
                .cmp(&a.1.quality)
                .then_with(|| context_window(b.1).cmp(&context_window(a.1)))
                .then_with(|| estimated_cost(a.1).total_cmp(&estimated_cost(b.1)))
                .then_with(|| a.0.cmp(b.0))
        }
        RoutingProfile::CloudOnly | RoutingProfile::Balanced => {
            quality_distance(a.1.quality, min_quality)
                .cmp(&quality_distance(b.1.quality, min_quality))
                .then_with(|| estimated_cost(a.1).total_cmp(&estimated_cost(b.1)))
                .then_with(|| a.1.local.cmp(&b.1.local).reverse())
                .then_with(|| a.0.cmp(b.0))
        }
    }
}

fn cascade_order<'a>(
    primary: &ModelConfig,
    candidates: &[(&'a String, &'a ModelConfig)],
) -> Vec<(&'a String, &'a ModelConfig)> {
    let mut fallbacks: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|(_, model)| model.quality >= primary.quality)
        .collect();
    fallbacks.sort_by(|a, b| {
        a.1.quality
            .cmp(&b.1.quality)
            .then_with(|| estimated_cost(a.1).total_cmp(&estimated_cost(b.1)))
            .then_with(|| a.0.cmp(b.0))
    });
    fallbacks
}

fn quality_distance(quality: ModelQuality, required: ModelQuality) -> u8 {
    quality_rank(quality).saturating_sub(quality_rank(required))
}

fn quality_rank(quality: ModelQuality) -> u8 {
    match quality {
        ModelQuality::Basic => 0,
        ModelQuality::Standard => 1,
        ModelQuality::Strong => 2,
        ModelQuality::Premium => 3,
    }
}

fn estimated_cost(model: &ModelConfig) -> f64 {
    model.input_per_m + model.output_per_m
}

fn context_window(model: &ModelConfig) -> usize {
    model.context_window.unwrap_or(usize::MAX)
}

fn profile_name(profile: RoutingProfile) -> &'static str {
    match profile {
        RoutingProfile::LocalFirst => "local-first",
        RoutingProfile::Balanced => "balanced",
        RoutingProfile::BestQuality => "best-quality",
        RoutingProfile::CloudOnly => "cloud-only",
    }
}

fn difficulty_name(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "easy",
        Difficulty::Standard => "standard",
        Difficulty::Hard => "hard",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Decision, HeaderPredicate, ModelConfig, Predicate, ProviderKind, RoutingConfig, RuleSpec,
    };
    use crate::types::Message;
    use std::collections::{BTreeMap, HashMap};

    fn cfg(profile: RoutingProfile) -> Config {
        let mut models = BTreeMap::new();
        models.insert(
            "tiny-local".into(),
            ModelConfig {
                provider: ProviderKind::Ollama,
                model_id: "llama3.2:3b".into(),
                input_per_m: 0.0,
                output_per_m: 0.0,
                quality: ModelQuality::Basic,
                context_window: Some(8_192),
                supports_tools: false,
                supports_vision: false,
                local: true,
                enabled: true,
            },
        );
        models.insert(
            "sonnet".into(),
            ModelConfig {
                provider: ProviderKind::Anthropic,
                model_id: "claude-sonnet".into(),
                input_per_m: 3.0,
                output_per_m: 15.0,
                quality: ModelQuality::Strong,
                context_window: Some(200_000),
                supports_tools: true,
                supports_vision: true,
                local: false,
                enabled: true,
            },
        );

        Config {
            server: Default::default(),
            ledger: Default::default(),
            providers: Default::default(),
            models,
            rules: Vec::new(),
            cascade: Default::default(),
            routing: RoutingConfig {
                profile,
                ..Default::default()
            },
        }
    }

    fn req(content: &str) -> Request {
        Request {
            requested_model: "auto".into(),
            messages: vec![Message {
                role: Role::User,
                content: content.into(),
                name: None,
                extra: HashMap::new(),
            }],
            tools: Vec::new(),
            stream: false,
            temperature: None,
            max_tokens: None,
            headers: HashMap::new(),
        }
    }

    #[test]
    fn local_first_uses_local_for_easy_prompt() {
        let decision = decide(&cfg(RoutingProfile::LocalFirst), &req("hello")).unwrap();
        assert_eq!(decision.primary_alias, "tiny-local");
    }

    #[test]
    fn hard_prompt_uses_strong_model() {
        let decision = decide(
            &cfg(RoutingProfile::Balanced),
            &req("debug this distributed race condition"),
        )
        .unwrap();
        assert_eq!(decision.primary_alias, "sonnet");
    }

    #[test]
    fn header_override_must_name_known_alias() {
        let mut cfg = cfg(RoutingProfile::Balanced);
        cfg.rules.push(RuleSpec {
            name: "override".into(),
            when: Predicate {
                always: None,
                has_tools: None,
                tokens_in_gt: None,
                tokens_in_lt: None,
                system_matches: None,
                model_alias_eq: None,
                header: Some(HeaderPredicate {
                    name: "x-pirouter-route".into(),
                    equals: None,
                    any_value: true,
                }),
            },
            then: Decision {
                primary: "$header:x-pirouter-route".into(),
                cascade: Vec::new(),
            },
        });
        let mut req = req("hello");
        req.headers
            .insert("x-pirouter-route".into(), "missing".into());

        let err = decide(&cfg, &req).unwrap_err().to_string();
        assert!(err.contains("unknown model alias `missing`"));
    }

    #[test]
    fn difficulty_header_overrides_heuristic() {
        let mut req = req("debug this distributed race condition");
        req.headers
            .insert("x-pirouter-difficulty".into(), "easy".into());
        let decision = decide(&cfg(RoutingProfile::LocalFirst), &req).unwrap();
        assert_eq!(decision.primary_alias, "tiny-local");
    }
}

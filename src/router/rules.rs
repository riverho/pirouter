//! Rule evaluation.
//!
//! Walks the configured rules in order, returns the first match as a
//! `ResolvedDecision`. Pure — no I/O. The resolved decision still names
//! aliases (not provider model IDs); the cascade executor does the alias
//! lookup at call time.

use crate::config::{Config, Predicate, RuleSpec};
use crate::types::Request;
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::RwLock;

/// The output of rule evaluation: which rule matched, the primary alias
/// (already resolved if it was a `$header:` variable), and the cascade.
#[derive(Debug, Clone)]
pub struct ResolvedDecision {
    pub rule_name: String,
    pub primary_alias: String,
    pub cascade_aliases: Vec<String>,
}

/// Tiny in-process cache for compiled regexes, keyed by pattern. Config
/// validation catches invalid patterns up front; the cache keeps request-time
/// rule evaluation from recompiling the same expression repeatedly.
static REGEX_CACHE: Lazy<RwLock<HashMap<String, Regex>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

fn compile_regex(pat: &str) -> Result<Regex> {
    if let Some(r) = REGEX_CACHE.read().unwrap().get(pat) {
        return Ok(r.clone());
    }
    let mut cache = REGEX_CACHE.write().unwrap();
    if let Some(r) = cache.get(pat) {
        return Ok(r.clone());
    }
    let r = Regex::new(pat).map_err(|e| anyhow!("invalid regex `{pat}`: {e}"))?;
    cache.insert(pat.to_string(), r.clone());
    Ok(r)
}

/// Evaluate rules against a request and return the first match.
pub fn decide(cfg: &Config, req: &Request) -> Result<ResolvedDecision> {
    for rule in &cfg.rules {
        if matches(&rule.when, req)? {
            return Ok(resolve(rule, req));
        }
    }
    Err(anyhow!(
        "no rule matched (add a default rule with `when = {{ always = true }}`)"
    ))
}

fn matches(pred: &Predicate, req: &Request) -> Result<bool> {
    if pred.always == Some(true) {
        return Ok(true);
    }
    if let Some(want) = pred.has_tools {
        if req.has_tools() != want {
            return Ok(false);
        }
    }
    if let Some(n) = pred.tokens_in_gt {
        if req.estimate_input_tokens() <= n {
            return Ok(false);
        }
    }
    if let Some(n) = pred.tokens_in_lt {
        if req.estimate_input_tokens() >= n {
            return Ok(false);
        }
    }
    if let Some(pat) = &pred.system_matches {
        let re = compile_regex(pat)?;
        if !re.is_match(&req.system_text()) {
            return Ok(false);
        }
    }
    if let Some(alias) = &pred.model_alias_eq {
        if &req.requested_model != alias {
            return Ok(false);
        }
    }
    if let Some(h) = &pred.header {
        let name = h.name.to_lowercase();
        match (req.headers.get(&name), &h.equals, h.any_value) {
            (Some(v), Some(want), _) if v == want => {}
            (Some(v), None, true) if !v.is_empty() => {}
            _ => return Ok(false),
        }
    }
    // If every present predicate matched (and at least one was set),
    // we're good. An empty Predicate would always match, which is why
    // we require `always = true` to be explicit.
    Ok(pred.has_any_condition())
}

fn resolve(rule: &RuleSpec, req: &Request) -> ResolvedDecision {
    let primary = resolve_target(&rule.then.primary, req);
    let cascade = rule
        .then
        .cascade
        .iter()
        .map(|t| resolve_target(t, req))
        .collect();
    ResolvedDecision {
        rule_name: rule.name.clone(),
        primary_alias: primary,
        cascade_aliases: cascade,
    }
}

/// Resolve a target spec like `$header:x-pirouter-route` into a concrete
/// alias. Returns the literal string if not a variable.
fn resolve_target(spec: &str, req: &Request) -> String {
    if let Some(name) = spec.strip_prefix("$header:") {
        return req
            .headers
            .get(&name.to_lowercase())
            .cloned()
            .unwrap_or_else(|| spec.to_string());
    }
    spec.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Decision, HeaderPredicate};
    use crate::types::{Message, Role};
    use std::collections::HashMap;

    fn empty_pred() -> Predicate {
        Predicate {
            always: None,
            has_tools: None,
            tokens_in_gt: None,
            tokens_in_lt: None,
            system_matches: None,
            model_alias_eq: None,
            header: None,
        }
    }

    fn req_with(system: &str) -> Request {
        Request {
            requested_model: "auto".into(),
            messages: vec![Message {
                role: Role::System,
                content: system.into(),
                name: None,
                extra: HashMap::new(),
            }],
            tools: vec![],
            stream: false,
            temperature: None,
            max_tokens: None,
            headers: HashMap::new(),
        }
    }

    #[test]
    fn always_rule_matches_everything() {
        let pred = Predicate {
            always: Some(true),
            ..empty_pred()
        };
        assert!(matches(&pred, &req_with("")).unwrap());
    }

    #[test]
    fn system_regex_matches() {
        let pred = Predicate {
            system_matches: Some("(?i)coding".into()),
            ..empty_pred()
        };
        assert!(matches(&pred, &req_with("You are a Coding assistant")).unwrap());
        assert!(!matches(&pred, &req_with("You are a poet")).unwrap());
    }

    #[test]
    fn header_variable_resolves() {
        let mut req = req_with("");
        req.headers.insert("x-pirouter-route".into(), "opus".into());
        let rule = RuleSpec {
            name: "override".into(),
            when: Predicate {
                always: Some(true),
                ..empty_pred()
            },
            then: Decision {
                primary: "$header:x-pirouter-route".into(),
                cascade: vec![],
            },
        };
        let d = resolve(&rule, &req);
        assert_eq!(d.primary_alias, "opus");
    }

    #[test]
    fn header_predicate_with_any_value() {
        let mut req = req_with("");
        req.headers.insert("x-foo".into(), "bar".into());
        let pred = Predicate {
            header: Some(HeaderPredicate {
                name: "x-foo".into(),
                equals: None,
                any_value: true,
            }),
            ..empty_pred()
        };
        assert!(matches(&pred, &req).unwrap());
    }
}

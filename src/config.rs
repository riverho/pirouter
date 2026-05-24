//! Configuration loading.
//!
//! Reads TOML from a platform-appropriate path (or `--config` override),
//! validates routing structure up front, and produces a `Config` that the rest
//! of the daemon treats as immutable for the lifetime of a server instance.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub ledger: LedgerConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    /// Model aliases. Order in the TOML is not significant.
    #[serde(default)]
    pub models: BTreeMap<String, ModelConfig>,
    #[serde(default)]
    pub rules: Vec<RuleSpec>,
    #[serde(default)]
    pub cascade: CascadeConfig,
    #[serde(default)]
    pub routing: RoutingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            log_level: default_log_level(),
        }
    }
}

fn default_bind() -> String {
    // Keep pirouter adjacent to, but not conflicting with, Ollama's default
    // 127.0.0.1:11434 listener.
    "127.0.0.1:11435".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LedgerConfig {
    /// Override for the SQLite path. Defaults to the platform data dir.
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProvidersConfig {
    pub anthropic: Option<ProviderCreds>,
    pub openai: Option<ProviderCreds>,
    pub ollama: Option<ProviderCreds>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderCreds {
    /// Either a literal key (`api_key`) or an env var name (`api_key_env`).
    /// `api_key_env` is preferred — keeps secrets out of the config file.
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    pub base_url: String,
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,
}

impl ProviderCreds {
    pub fn resolved_key(&self) -> Result<String> {
        // Prefer the environment if both are present. That lets users keep a
        // placeholder or stale key in config without accidentally shadowing
        // their shell/service-manager secret.
        if let Some(env) = &self.api_key_env {
            return std::env::var(env).with_context(|| format!("env var {env} not set"));
        }
        if let Some(k) = &self.api_key {
            return Ok(k.clone());
        }
        Err(anyhow!("provider requires api_key or api_key_env"))
    }
}

fn default_request_timeout_secs() -> u64 {
    120
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub provider: ProviderKind,
    pub model_id: String,
    /// USD per 1M input tokens.
    #[serde(default)]
    pub input_per_m: f64,
    /// USD per 1M output tokens.
    #[serde(default)]
    pub output_per_m: f64,
    /// Human routing tier. The policy router uses this to select the lowest
    /// adequate model before escalating.
    #[serde(default)]
    pub quality: ModelQuality,
    /// Advertised context window. Unknown means "do not filter by context."
    #[serde(default)]
    pub context_window: Option<usize>,
    #[serde(default)]
    pub supports_tools: bool,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub local: bool,
    /// Disable without deleting from config or GUI-managed catalogs.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Anthropic,
    Ollama,
    Openai,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Ollama => "ollama",
            ProviderKind::Openai => "openai",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum ModelQuality {
    Basic,
    Standard,
    Strong,
    Premium,
}

impl Default for ModelQuality {
    fn default() -> Self {
        Self::Standard
    }
}

/// A single rule. The TOML representation is a `[[rules]]` table.
#[derive(Debug, Clone, Deserialize)]
pub struct RuleSpec {
    pub name: String,
    pub when: Predicate,
    pub then: Decision,
}

/// All predicates a rule can match on. Exactly one variant is set per rule
/// in v0 — composition can be added later by making this a `Vec<Predicate>`
/// with `all`/`any` combinators.
#[derive(Debug, Clone, Deserialize)]
pub struct Predicate {
    #[serde(default)]
    pub always: Option<bool>,
    #[serde(default)]
    pub has_tools: Option<bool>,
    #[serde(default)]
    pub tokens_in_gt: Option<usize>,
    #[serde(default)]
    pub tokens_in_lt: Option<usize>,
    /// Regex (RE2-ish via the `regex` crate) tested against concatenated
    /// system messages.
    #[serde(default)]
    pub system_matches: Option<String>,
    #[serde(default)]
    pub model_alias_eq: Option<String>,
    #[serde(default)]
    pub header: Option<HeaderPredicate>,
}

impl Predicate {
    pub fn has_any_condition(&self) -> bool {
        self.has_tools.is_some()
            || self.tokens_in_gt.is_some()
            || self.tokens_in_lt.is_some()
            || self.system_matches.is_some()
            || self.model_alias_eq.is_some()
            || self.header.is_some()
            || self.always == Some(true)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeaderPredicate {
    pub name: String,
    #[serde(default)]
    pub equals: Option<String>,
    /// If true, match any non-empty value.
    #[serde(default)]
    pub any_value: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Decision {
    /// Model alias to use first. Special form `"$header:foo"` reads the
    /// alias from request header `foo` (lower-cased).
    pub primary: String,
    /// Ordered list of escalation targets. Empty means "no cascade".
    #[serde(default)]
    pub cascade: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CascadeConfig {
    #[serde(default = "default_true")]
    pub on_http_error: bool,
    #[serde(default)]
    pub on_short_response: bool,
    #[serde(default = "default_min_output")]
    pub min_output_tokens: u32,
    /// Opt-in marker pattern: if the response contains this string, the
    /// cascade escalates.
    #[serde(default)]
    pub on_marker: Option<String>,
}

impl Default for CascadeConfig {
    fn default() -> Self {
        Self {
            on_http_error: true,
            on_short_response: false,
            min_output_tokens: 8,
            on_marker: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_min_output() -> u32 {
    8
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoutingConfig {
    /// Controls the built-in policy router used when no explicit rule matches
    /// or when `[rules]` is intentionally empty.
    #[serde(default)]
    pub profile: RoutingProfile,
    /// Include stronger candidates after the primary so cascade can recover
    /// from weak answers or provider errors.
    #[serde(default = "default_true")]
    pub auto_cascade: bool,
    /// Number of fallback aliases generated by the policy router.
    #[serde(default = "default_policy_fallbacks")]
    pub max_policy_fallbacks: usize,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            profile: RoutingProfile::Balanced,
            auto_cascade: true,
            max_policy_fallbacks: default_policy_fallbacks(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RoutingProfile {
    LocalFirst,
    Balanced,
    BestQuality,
    CloudOnly,
}

impl Default for RoutingProfile {
    fn default() -> Self {
        Self::Balanced
    }
}

fn default_policy_fallbacks() -> usize {
    3
}

impl Config {
    /// Load from a TOML file. Validates that every rule references a
    /// declared model alias.
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading config at {}", path.display()))?;
        let cfg: Config = toml::from_str(&raw).context("parsing config TOML")?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        for rule in &self.rules {
            if !rule.when.has_any_condition() {
                return Err(anyhow!(
                    "rule `{}` has an empty predicate; use `always = true` for a default rule",
                    rule.name
                ));
            }
            if let Some(pattern) = &rule.when.system_matches {
                regex::Regex::new(pattern).map_err(|e| {
                    anyhow!(
                        "rule `{}` has invalid system_matches regex `{}`: {}",
                        rule.name,
                        pattern,
                        e
                    )
                })?;
            }
            // Skip header-variable models — they're resolved at request time.
            let aliases = std::iter::once(&rule.then.primary)
                .chain(rule.then.cascade.iter())
                .filter(|m| !m.starts_with("$header:"));
            for alias in aliases {
                if !self.models.contains_key(alias) {
                    return Err(anyhow!(
                        "rule `{}` references unknown model alias `{}`",
                        rule.name,
                        alias
                    ));
                }
            }
        }
        Ok(())
    }

    /// Default config location per platform.
    pub fn default_path() -> Option<PathBuf> {
        let dirs = directories::ProjectDirs::from("", "", "pirouter")?;
        Some(dirs.config_dir().join("config.toml"))
    }

    /// Default ledger path per platform (used when `[ledger].path` is
    /// unset).
    pub fn default_ledger_path() -> Option<PathBuf> {
        let dirs = directories::ProjectDirs::from("", "", "pirouter")?;
        Some(dirs.data_dir().join("ledger.db"))
    }
}

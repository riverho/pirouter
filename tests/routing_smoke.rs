//! Smoke tests for the rule engine.
//!
//! These exercise routing decisions without hitting any provider — the
//! whole point of separating rules from cascade is that we can test
//! decisions in isolation.

use pirouter::config::Config;
use pirouter::router::rules;
use pirouter::types::{Message, Request, Role};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn load_example_config() -> Config {
    // Use a minimal in-test config rather than parsing the example file,
    // so the test doesn't depend on cwd.
    let toml = r#"
[providers.anthropic]
api_key = "x"
base_url = "https://api.anthropic.com"

[models.haiku]
provider = "anthropic"
model_id = "claude-haiku-4-5-20251001"

[models.sonnet]
provider = "anthropic"
model_id = "claude-sonnet-4-6"

[models.opus]
provider = "anthropic"
model_id = "claude-opus-4-6"

[[rules]]
name = "tool-use"
when = { has_tools = true }
then = { primary = "sonnet", cascade = ["opus"] }

[[rules]]
name = "long-context"
when = { tokens_in_gt = 1000 }
then = { primary = "sonnet", cascade = ["opus"] }

[[rules]]
name = "default"
when = { always = true }
then = { primary = "haiku", cascade = ["sonnet"] }
"#;
    toml::from_str(toml).unwrap()
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
        tools: vec![],
        stream: false,
        temperature: None,
        max_tokens: None,
        headers: HashMap::new(),
    }
}

fn write_temp_config(contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("pirouter-test-{}.toml", ulid::Ulid::new()));
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn short_request_hits_default() {
    let cfg = load_example_config();
    let d = rules::decide(&cfg, &req("hello")).unwrap();
    assert_eq!(d.rule_name, "default");
    assert_eq!(d.primary_alias, "haiku");
    assert_eq!(d.cascade_aliases, vec!["sonnet".to_string()]);
}

#[test]
fn long_request_routes_to_sonnet() {
    let cfg = load_example_config();
    // ~5000 chars ≈ 1250 tokens, above the 1000 threshold
    let long = "x".repeat(5000);
    let d = rules::decide(&cfg, &req(&long)).unwrap();
    assert_eq!(d.rule_name, "long-context");
    assert_eq!(d.primary_alias, "sonnet");
}

#[test]
fn tool_use_takes_precedence_over_default() {
    let cfg = load_example_config();
    let mut r = req("hi");
    r.tools.push(pirouter::types::Tool {
        name: "search".into(),
        description: None,
        parameters: serde_json::json!({}),
    });
    let d = rules::decide(&cfg, &r).unwrap();
    assert_eq!(d.rule_name, "tool-use");
}

#[test]
fn validates_unknown_alias() {
    let bad = r#"
[models.haiku]
provider = "anthropic"
model_id = "x"

[[rules]]
name = "broken"
when = { always = true }
then = { primary = "does-not-exist", cascade = [] }
"#;
    let path = write_temp_config(bad);
    let err = Config::load(&path).unwrap_err().to_string();
    fs::remove_file(path).ok();
    assert!(err.contains("unknown model alias `does-not-exist`"));
}

#[test]
fn rejects_empty_predicate() {
    let bad = r#"
[models.haiku]
provider = "anthropic"
model_id = "x"

[[rules]]
name = "empty"
when = {}
then = { primary = "haiku", cascade = [] }
"#;
    let path = write_temp_config(bad);
    let err = Config::load(&path).unwrap_err().to_string();
    fs::remove_file(path).ok();
    assert!(err.contains("empty predicate"));
}

//! pirouter CLI.
//!
//! Subcommands:
//!   pirouter run             — start the daemon (blocking)
//!   pirouter check-config    — validate config and exit
//!   pirouter stats [--hours] — summarize ledger
//!
//! Config path resolution: --config flag > $PIROUTER_CONFIG > platform default.

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use pirouter::config::{Config, ModelQuality};
use pirouter::ledger::Ledger;
use pirouter::types::{Message, Request, Role, Tool};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "pirouter",
    version,
    about = "Lightweight LLM routing daemon",
    long_about = "pirouter sits between your agent and your LLM providers, routing each \
                  request to the cheapest model that can handle it."
)]
struct Cli {
    /// Path to config.toml (overrides $PIROUTER_CONFIG and platform default).
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start the routing daemon.
    Run,
    /// Validate config and exit non-zero on error.
    CheckConfig,
    /// Print the configured model catalog.
    Models,
    /// Dry-run the policy/router decision for a prompt.
    Route {
        /// Requested model alias from the client side.
        #[arg(long, default_value = "auto")]
        model: String,
        /// Simulate a request with tool definitions.
        #[arg(long)]
        tools: bool,
        /// Prompt text to classify and route.
        #[arg(long)]
        prompt: String,
    },
    /// Print a summary of the ledger.
    Stats {
        /// Look back this many hours.
        #[arg(long, default_value_t = 24)]
        hours: i64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let cfg_path = resolve_config_path(cli.config.clone())?;
    let cfg = Config::load(&cfg_path)
        .with_context(|| format!("loading config at {}", cfg_path.display()))?;

    init_tracing(&cfg);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        match cli.cmd {
            Cmd::Run => pirouter::server::serve(cfg).await,
            Cmd::CheckConfig => {
                println!("config OK: {}", cfg_path.display());
                println!("  models:    {}", cfg.models.len());
                println!("  rules:     {}", cfg.rules.len());
                Ok(())
            }
            Cmd::Models => {
                print_models(&cfg);
                Ok(())
            }
            Cmd::Route {
                model,
                tools,
                prompt,
            } => print_route(&cfg, model, tools, prompt),
            Cmd::Stats { hours } => {
                let ledger = Ledger::open(&cfg).await?;
                print_stats(&ledger, hours).await
            }
        }
    })
}

fn resolve_config_path(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    if let Ok(env) = std::env::var("PIROUTER_CONFIG") {
        return Ok(PathBuf::from(env));
    }
    Config::default_path().ok_or_else(|| {
        anyhow!(
            "no config path given and platform default unavailable; \
             pass --config or set PIROUTER_CONFIG"
        )
    })
}

fn init_tracing(cfg: &Config) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cfg.server.log_level));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

async fn print_stats(ledger: &Ledger, hours: i64) -> Result<()> {
    let summary = ledger.summary(hours).await?;
    if summary.is_empty() {
        println!("no requests in the last {hours} hour(s)");
        return Ok(());
    }
    println!(
        "{:<32} {:>8} {:>12} {:>12} {:>10} {:>10}",
        "model", "reqs", "in_tokens", "out_tokens", "cost_usd", "avg_ms"
    );
    println!("{}", "-".repeat(88));
    let mut total_cost = 0.0;
    let mut total_reqs = 0;
    for row in &summary {
        println!(
            "{:<32} {:>8} {:>12} {:>12} {:>10.4} {:>10.0}",
            truncate(&row.model, 32),
            row.requests,
            row.input_tokens,
            row.output_tokens,
            row.cost_usd,
            row.avg_latency_ms
        );
        total_cost += row.cost_usd;
        total_reqs += row.requests;
    }
    println!("{}", "-".repeat(88));
    println!(
        "{:<32} {:>8} {:>12} {:>12} {:>10.4}",
        "TOTAL", total_reqs, "", "", total_cost
    );
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max - 1).collect();
        format!("{head}…")
    }
}

fn print_models(cfg: &Config) {
    println!(
        "{:<24} {:<10} {:<22} {:<9} {:>8} {:>6} {:>6}",
        "alias", "provider", "model_id", "quality", "context", "tools", "local"
    );
    println!("{}", "-".repeat(92));
    for (alias, model) in &cfg.models {
        println!(
            "{:<24} {:<10} {:<22} {:<9} {:>8} {:>6} {:>6}",
            truncate(alias, 24),
            model.provider.to_string(),
            truncate(&model.model_id, 22),
            quality_name(model.quality),
            model
                .context_window
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".into()),
            yes_no(model.supports_tools),
            yes_no(model.local)
        );
    }
}

fn print_route(cfg: &Config, model: String, tools: bool, prompt: String) -> Result<()> {
    let mut request = Request {
        requested_model: model,
        messages: vec![Message {
            role: Role::User,
            content: prompt,
            name: None,
            extra: HashMap::new(),
        }],
        tools: Vec::new(),
        stream: false,
        temperature: None,
        max_tokens: None,
        headers: HashMap::new(),
    };

    if tools {
        request.tools.push(Tool {
            name: "tool".into(),
            description: Some("synthetic dry-run tool".into()),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        });
    }

    let decision = pirouter::router::policy::decide(cfg, &request)?;
    println!("rule:    {}", decision.rule_name);
    println!("primary: {}", decision.primary_alias);
    if decision.cascade_aliases.is_empty() {
        println!("cascade: <none>");
    } else {
        println!("cascade: {}", decision.cascade_aliases.join(" -> "));
    }
    Ok(())
}

fn quality_name(quality: ModelQuality) -> &'static str {
    match quality {
        ModelQuality::Basic => "basic",
        ModelQuality::Standard => "standard",
        ModelQuality::Strong => "strong",
        ModelQuality::Premium => "premium",
    }
}

fn yes_no(v: bool) -> &'static str {
    if v {
        "yes"
    } else {
        "no"
    }
}

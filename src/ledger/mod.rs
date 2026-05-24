//! Cost ledger — SQLite-backed log of every routed request.
//!
//! The ledger is the *observability surface* for routing decisions. Each
//! row carries the rule that matched, the cascade path (as JSON), token
//! counts, computed cost, and latency. `pirouter stats` slices it.

use crate::config::Config;
use crate::router::cascade::{AttemptOutcome, CascadeAttempt, CascadeOutcome};
use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;
use ulid::Ulid;

#[derive(Clone)]
pub struct Ledger {
    pool: SqlitePool,
}

#[derive(Debug)]
pub struct LedgerRow<'a> {
    pub requested_model: &'a str,
    pub rule: Option<&'a str>,
    pub primary_model: &'a str,
    pub final_model: &'a str,
    pub outcome: &'a CascadeOutcome,
    pub total_latency_ms: u64,
    pub status: &'a str,
}

impl Ledger {
    pub async fn open(cfg: &Config) -> Result<Self> {
        let path = resolve_path(cfg)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .with_context(|| format!("opening ledger at {}", path.display()))?;
        sqlx::query(include_str!("schema.sql"))
            .execute(&pool)
            .await
            .context("applying ledger schema")?;
        sqlx::query("ALTER TABLE requests ADD COLUMN requested_model TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await
            .context("enabling ledger WAL mode")?;
        Ok(Self { pool })
    }

    pub async fn record(&self, cfg: &Config, row: LedgerRow<'_>) -> Result<()> {
        let id = Ulid::new().to_string();
        let ts = Utc::now().timestamp();
        let cascade_path =
            serde_json::to_string(&row.outcome.attempts).unwrap_or_else(|_| "[]".into());

        let (input_tokens, output_tokens, cost_usd) = compute_route_usage(cfg, row.outcome);

        sqlx::query(
            "INSERT INTO requests (id, ts, requested_model, route_rule, primary_model, final_model, \
             cascade_path, input_tokens, output_tokens, cost_usd, latency_ms, status) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(ts)
        .bind(row.requested_model)
        .bind(row.rule)
        .bind(row.primary_model)
        .bind(row.final_model)
        .bind(&cascade_path)
        .bind(input_tokens as i64)
        .bind(output_tokens as i64)
        .bind(cost_usd)
        .bind(row.total_latency_ms as i64)
        .bind(row.status)
        .execute(&self.pool)
        .await
        .context("writing ledger row")?;
        Ok(())
    }

    /// Aggregate stats over the last `hours` hours.
    pub async fn summary(&self, hours: i64) -> Result<Vec<ModelSummary>> {
        let since = Utc::now().timestamp() - hours * 3600;
        let rows: Vec<(String, i64, i64, i64, f64, f64)> = sqlx::query_as(
            "SELECT final_model, COUNT(*) as n, \
                    COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), \
                    COALESCE(SUM(cost_usd),0.0), COALESCE(AVG(latency_ms),0.0) \
             FROM requests WHERE ts >= ? GROUP BY final_model ORDER BY n DESC",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .context("ledger summary")?;
        Ok(rows
            .into_iter()
            .map(|(m, n, i, o, c, l)| ModelSummary {
                model: m,
                requests: n,
                input_tokens: i,
                output_tokens: o,
                cost_usd: c,
                avg_latency_ms: l,
            })
            .collect())
    }
}

#[derive(Debug)]
pub struct ModelSummary {
    pub model: String,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f64,
    pub avg_latency_ms: f64,
}

fn resolve_path(cfg: &Config) -> Result<PathBuf> {
    if let Some(p) = &cfg.ledger.path {
        return Ok(p.clone());
    }
    Config::default_ledger_path()
        .context("could not determine default ledger path on this platform")
}

fn compute_route_usage(cfg: &Config, outcome: &CascadeOutcome) -> (u32, u32, f64) {
    outcome.attempts.iter().fold(
        (0, 0, 0.0),
        |(input_total, output_total, cost_total), attempt| {
            let (input_tokens, output_tokens) = attempt_usage(attempt);
            (
                input_total + input_tokens,
                output_total + output_tokens,
                cost_total + compute_cost(cfg, &attempt.alias, input_tokens, output_tokens),
            )
        },
    )
}

fn attempt_usage(attempt: &CascadeAttempt) -> (u32, u32) {
    match &attempt.outcome {
        AttemptOutcome::Ok {
            input_tokens,
            output_tokens,
        }
        | AttemptOutcome::EscalatedShortResponse {
            input_tokens,
            output_tokens,
        }
        | AttemptOutcome::EscalatedMarker {
            input_tokens,
            output_tokens,
        } => (*input_tokens, *output_tokens),
        AttemptOutcome::EscalatedHttpError { .. } => (0, 0),
    }
}

fn compute_cost(cfg: &Config, alias: &str, input_tokens: u32, output_tokens: u32) -> f64 {
    let pricing = cfg.models.get(alias);
    let Some(m) = pricing else {
        return 0.0;
    };
    let input_cost = (input_tokens as f64) * m.input_per_m / 1_000_000.0;
    let output_cost = (output_tokens as f64) * m.output_per_m / 1_000_000.0;
    input_cost + output_cost
}

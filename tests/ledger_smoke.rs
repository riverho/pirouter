use pirouter::config::{
    CascadeConfig, Config, LedgerConfig, ModelConfig, ModelQuality, ProviderKind, ProvidersConfig,
    RoutingConfig, ServerConfig,
};
use pirouter::ledger::{Ledger, LedgerRow};
use pirouter::router::cascade::{AttemptOutcome, CascadeAttempt, CascadeOutcome};
use pirouter::types::Response;
use std::collections::BTreeMap;

fn config_with_ledger_path(path: std::path::PathBuf) -> Config {
    let mut models = BTreeMap::new();
    models.insert(
        "cheap".into(),
        ModelConfig {
            provider: ProviderKind::Openai,
            model_id: "provider-model-request".into(),
            input_per_m: 1.0,
            output_per_m: 2.0,
            quality: ModelQuality::Standard,
            context_window: Some(128_000),
            supports_tools: true,
            supports_vision: false,
            local: false,
            enabled: true,
        },
    );

    Config {
        server: ServerConfig::default(),
        ledger: LedgerConfig { path: Some(path) },
        providers: ProvidersConfig::default(),
        models,
        rules: Vec::new(),
        cascade: CascadeConfig::default(),
        routing: RoutingConfig::default(),
    }
}

#[tokio::test]
async fn ledger_prices_attempts_by_alias() {
    let path = std::env::temp_dir().join(format!("pirouter-ledger-{}.db", ulid::Ulid::new()));
    let cfg = config_with_ledger_path(path.clone());
    let ledger = Ledger::open(&cfg).await.unwrap();

    let outcome = CascadeOutcome {
        attempts: vec![CascadeAttempt {
            alias: "cheap".into(),
            provider: "openai".into(),
            model_id: "provider-returned-different-id".into(),
            latency_ms: 10,
            outcome: AttemptOutcome::Ok {
                input_tokens: 1_000_000,
                output_tokens: 1_000_000,
            },
        }],
        final_response: Some(Response {
            model: "provider-returned-different-id".into(),
            content: "ok".into(),
            finish_reason: Some("stop".into()),
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            tool_calls: Vec::new(),
        }),
        final_alias: Some("cheap".into()),
    };

    ledger
        .record(
            &cfg,
            LedgerRow {
                requested_model: "auto",
                rule: Some("test"),
                primary_model: "cheap",
                final_model: "cheap",
                outcome: &outcome,
                total_latency_ms: 10,
                status: "ok",
            },
        )
        .await
        .unwrap();

    let summary = ledger.summary(1).await.unwrap();
    assert_eq!(summary.len(), 1);
    assert_eq!(summary[0].model, "cheap");
    assert_eq!(summary[0].input_tokens, 1_000_000);
    assert_eq!(summary[0].output_tokens, 1_000_000);
    assert!((summary[0].cost_usd - 3.0).abs() < f64::EPSILON);

    drop(ledger);
    std::fs::remove_file(&path).ok();
    std::fs::remove_file(path.with_extension("db-wal")).ok();
    std::fs::remove_file(path.with_extension("db-shm")).ok();
}

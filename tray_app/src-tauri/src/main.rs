#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{DateTime, Local, TimeZone};
use pirouter::config::{
    Config, ModelQuality, Predicate, ProviderCreds, ProviderKind, RoutingProfile,
};
use serde::Serialize;
use sqlx::sqlite::SqlitePoolOptions;
use std::{
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::Mutex,
    time::Duration,
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    path::BaseDirectory,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

#[derive(Default)]
struct DaemonProcess {
    child: Mutex<Option<Child>>,
}

// ── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStateDto {
    daemon: DaemonDto,
    routing: RoutingDto,
    providers: Vec<ProviderDto>,
    models: Vec<ModelDto>,
    rules: Vec<RuleDto>,
    ledger: Vec<LedgerRowDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DaemonDto {
    status: String,
    mode: String,
    endpoint: String,
    health_url: String,
    bind: String,
    config_path: String,
    ledger_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoutingDto {
    profile: String,
    auto_cascade: bool,
    max_fallbacks: usize,
    on_http_error: bool,
    on_short_response: bool,
    min_output_tokens: u32,
    marker: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderDto {
    id: String,
    name: String,
    base_url: String,
    key_env: String,
    timeout: u64,
    enabled: bool,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelDto {
    id: String,
    alias: String,
    provider: String,
    model_id: String,
    quality: String,
    ctx: usize,
    tools: bool,
    vision: bool,
    cost_in: f64,
    cost_out: f64,
    enabled: bool,
    local: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuleDto {
    id: String,
    name: String,
    predicate: String,
    target: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LedgerRowDto {
    id: String,
    ts: String,
    requested: String,
    final_model: String,
    rule: String,
    status: String,
    input_tokens: i64,
    output_tokens: i64,
    cost: f64,
    latency: i64,
    cascade: Vec<CascadeAttemptDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CascadeAttemptDto {
    attempt: usize,
    alias: String,
    provider: String,
    model_id: String,
    latency: i64,
    outcome: String,
}

#[derive(Debug, sqlx::FromRow)]
struct LedgerDbRow {
    id: String,
    ts: i64,
    requested_model: String,
    route_rule: Option<String>,
    primary_model: String,
    final_model: String,
    cascade_path: String,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f64,
    latency_ms: i64,
    status: String,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
async fn load_app_state() -> Result<AppStateDto, String> {
    let (config_path, cfg) = load_config()?;
    let ledger_path = ledger_path(&cfg)?;
    let status = daemon_status(&cfg.server.bind).await;
    let endpoint = endpoint_from_bind(&cfg.server.bind);

    Ok(AppStateDto {
        daemon: DaemonDto {
            status,
            mode: mode_label(),
            endpoint,
            health_url: health_url_from_bind(&cfg.server.bind),
            bind: cfg.server.bind.clone(),
            config_path: display_path(&config_path),
            ledger_path: display_path(&ledger_path),
        },
        routing: RoutingDto {
            profile: routing_profile_label(cfg.routing.profile),
            auto_cascade: cfg.routing.auto_cascade,
            max_fallbacks: cfg.routing.max_policy_fallbacks,
            on_http_error: cfg.cascade.on_http_error,
            on_short_response: cfg.cascade.on_short_response,
            min_output_tokens: cfg.cascade.min_output_tokens,
            marker: cfg.cascade.on_marker.clone().unwrap_or_default(),
        },
        providers: provider_rows(&cfg),
        models: model_rows(&cfg),
        rules: rule_rows(&cfg),
        ledger: ledger_rows(&cfg, &ledger_path).await.unwrap_or_default(),
    })
}

#[tauri::command]
async fn daemon_action(app: AppHandle, action: String) -> Result<String, String> {
    match action.as_str() {
        "start" => start_daemon(&app),
        "stop" => stop_daemon(&app),
        "restart" => restart_daemon(&app).await,
        other => Err(format!("unknown daemon action `{other}`")),
    }
}

#[tauri::command]
fn validate_config() -> Result<String, String> {
    let (path, _) = load_config()?;
    Ok(format!("Config is valid: {}", path.display()))
}

#[tauri::command]
async fn test_provider(provider_id: String) -> Result<String, String> {
    let (_, cfg) = load_config()?;
    let provider = match provider_id.as_str() {
        "anthropic" => cfg.providers.anthropic.as_ref(),
        "openai" => cfg.providers.openai.as_ref(),
        "ollama" => cfg.providers.ollama.as_ref(),
        _ => return Err(format!("unknown provider `{provider_id}`")),
    }
    .ok_or_else(|| format!("provider `{provider_id}` is not configured"))?;

    let health_url = provider_health_url(provider);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(
            provider.request_timeout_secs.clamp(2, 15),
        ))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(&health_url)
        .headers(provider_headers(&provider_id, provider))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status().is_success() {
        Ok(format!("{provider_id} reachable at {health_url}"))
    } else {
        Err(format!(
            "{provider_id} returned HTTP {} at {health_url}",
            response.status()
        ))
    }
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    let _ = stop_daemon(&app);
    app.exit(0);
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .manage(DaemonProcess::default())
        .setup(|app| {
            setup_tray(app.handle())?;
            if is_background_launch() {
                let app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = start_daemon(&app);
                });
            } else {
                show_main_window(app.handle());
            }
            Ok(())
        })
        // Close button hides the window to tray instead of quitting.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_app_state,
            daemon_action,
            validate_config,
            test_provider,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running pirouter desktop");
}

// ── Tray ──────────────────────────────────────────────────────────────────────

/// Build a fresh tray menu reflecting current startup state.
/// Called once on setup and again after toggling "Start on Login."
fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let open = MenuItem::with_id(app, "open", "Open Settings", true, None::<&str>)?;
    let start = MenuItem::with_id(app, "start", "Start Daemon", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Stop Daemon", true, None::<&str>)?;
    let restart = MenuItem::with_id(app, "restart", "Restart Daemon", true, None::<&str>)?;
    let startup = CheckMenuItem::with_id(
        app,
        "startup",
        "Start on Login",
        true,
        is_startup_enabled(),
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &open,
            &PredefinedMenuItem::separator(app)?,
            &start,
            &stop,
            &restart,
            &PredefinedMenuItem::separator(app)?,
            &startup,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_tray_menu(app)?;

    TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "start" => {
                let _ = start_daemon(app);
            }
            "stop" => {
                let _ = stop_daemon(app);
            }
            "restart" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = restart_daemon(&app).await;
                });
            }
            "startup" => {
                toggle_startup();
                // Rebuild menu so the checkmark reflects the new state immediately.
                if let Some(tray) = app.tray_by_id("main") {
                    if let Ok(menu) = build_tray_menu(app) {
                        let _ = tray.set_menu(Some(menu));
                    }
                }
            }
            "quit" => quit_app(app.clone()),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

// ── Startup registry (Windows) ────────────────────────────────────────────────

/// Returns true if pirouter is registered in the current-user Run key.
#[cfg(windows)]
fn is_startup_enabled() -> bool {
    Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "pirouter",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_startup_enabled() -> bool {
    false
}

/// Adds or removes the startup registry entry for the current executable.
#[cfg(windows)]
fn toggle_startup() {
    if is_startup_enabled() {
        let _ = Command::new("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "pirouter",
                "/f",
            ])
            .output();
    } else if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "pirouter",
                "/t",
                "REG_SZ",
                "/d",
                &format!("\"{}\" --background", exe.display()),
                "/f",
            ])
            .output();
    }
}

#[cfg(not(windows))]
fn toggle_startup() {}

// ── Daemon helpers ────────────────────────────────────────────────────────────

fn start_daemon(app: &AppHandle) -> Result<String, String> {
    if owned_daemon_running(app)? {
        return Ok("daemon is already running from this tray app".to_string());
    }

    if configured_daemon_port_is_open() {
        return Ok("daemon is already running; leaving external process alone".to_string());
    }

    let bin = find_daemon_binary(app).ok_or_else(|| {
        "could not find pirouter daemon binary; build pirouter.exe before starting from the GUI"
            .to_string()
    })?;
    let mut command = Command::new(&bin);
    command.arg("run");
    if let Some(config_path) = std::env::var_os("PIROUTER_CONFIG") {
        command.arg("--config").arg(config_path);
    }
    hide_child_window(&mut command);
    let child = command
        .spawn()
        .map_err(|e| format!("starting {} failed: {e}", bin.display()))?;

    let pid = child.id();
    let state = app.state::<DaemonProcess>();
    let mut managed_child = state.child.lock().map_err(|e| e.to_string())?;
    *managed_child = Some(child);

    Ok(format!("started {} (pid {pid})", bin.display()))
}

async fn restart_daemon(app: &AppHandle) -> Result<String, String> {
    let stop_result = stop_daemon(app)?;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let start_result = start_daemon(app)?;
    Ok(format!("{stop_result}; {start_result}"))
}

fn stop_daemon(app: &AppHandle) -> Result<String, String> {
    let state = app.state::<DaemonProcess>();
    {
        let mut managed_child = state.child.lock().map_err(|e| e.to_string())?;
        if let Some(child) = managed_child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    *managed_child = None;
                    return Ok(format!("daemon already exited with {status}"));
                }
                Ok(None) => {
                    let pid = child.id();
                    child
                        .kill()
                        .map_err(|e| format!("stopping daemon pid {pid} failed: {e}"))?;
                    let _ = child.wait();
                    *managed_child = None;
                    return Ok(format!("stopped app-managed daemon pid {pid}"));
                }
                Err(error) => {
                    *managed_child = None;
                    return Err(format!("checking daemon process failed: {error}"));
                }
            }
        }
    }

    if configured_daemon_port_is_open() {
        return Err(
            "daemon is running but was not started by this tray app; leaving external process alone"
                .to_string(),
        );
    }

    Ok("daemon is not running".to_string())
}

fn owned_daemon_running(app: &AppHandle) -> Result<bool, String> {
    let state = app.state::<DaemonProcess>();
    let mut managed_child = state.child.lock().map_err(|e| e.to_string())?;
    if let Some(child) = managed_child.as_mut() {
        match child.try_wait() {
            Ok(Some(_)) => {
                *managed_child = None;
                Ok(false)
            }
            Ok(None) => Ok(true),
            Err(error) => {
                *managed_child = None;
                Err(format!("checking daemon process failed: {error}"))
            }
        }
    } else {
        Ok(false)
    }
}

fn configured_daemon_port_is_open() -> bool {
    let Ok((_, cfg)) = load_config() else {
        return false;
    };
    let bind = reachable_bind(&cfg.server.bind);
    let Ok(addrs) = bind.to_socket_addrs() else {
        return false;
    };
    addrs
        .into_iter()
        .any(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok())
}

fn is_background_launch() -> bool {
    std::env::args()
        .skip(1)
        .any(|arg| arg == "--background" || arg == "--tray")
}

fn find_daemon_binary(app: &AppHandle) -> Option<PathBuf> {
    let exe_name = if cfg!(windows) {
        "pirouter.exe"
    } else {
        "pirouter"
    };

    let mut candidates = Vec::new();
    if let Ok(resource) = app
        .path()
        .resolve(format!("binaries/{exe_name}"), BaseDirectory::Resource)
    {
        candidates.push(resource);
    }
    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            candidates.push(dir.join(exe_name));
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest.join("binaries").join(exe_name));
    candidates.push(
        manifest
            .join("..")
            .join("..")
            .join("target")
            .join("release")
            .join(exe_name),
    );
    candidates.push(
        manifest
            .join("..")
            .join("..")
            .join("target")
            .join("debug")
            .join(exe_name),
    );

    candidates.into_iter().find(|path| path.exists())
}

fn hide_child_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        command.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

// ── Config helpers ────────────────────────────────────────────────────────────

fn load_config() -> Result<(PathBuf, Config), String> {
    let path = std::env::var_os("PIROUTER_CONFIG")
        .map(PathBuf::from)
        .or_else(Config::default_path)
        .ok_or_else(|| "could not determine pirouter config path".to_string())?;
    let cfg = Config::load(&path).map_err(|e| e.to_string())?;
    Ok((path, cfg))
}

fn ledger_path(cfg: &Config) -> Result<PathBuf, String> {
    cfg.ledger
        .path
        .clone()
        .or_else(Config::default_ledger_path)
        .ok_or_else(|| "could not determine pirouter ledger path".to_string())
}

// ── URL helpers ───────────────────────────────────────────────────────────────

fn endpoint_from_bind(bind: &str) -> String {
    format!("http://{}/v1", reachable_bind(bind))
}

fn health_url_from_bind(bind: &str) -> String {
    format!("http://{}/healthz", reachable_bind(bind))
}

fn reachable_bind(bind: &str) -> String {
    bind.replacen("0.0.0.0:", "127.0.0.1:", 1)
        .replacen("[::]:", "127.0.0.1:", 1)
}

async fn daemon_status(bind: &str) -> String {
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_millis(900))
        .build()
    else {
        return "error".to_string();
    };
    match client.get(health_url_from_bind(bind)).send().await {
        Ok(response) if response.status().is_success() => "running".to_string(),
        Ok(_) => "error".to_string(),
        Err(_) => "stopped".to_string(),
    }
}

fn provider_health_url(creds: &ProviderCreds) -> String {
    let base = creds.base_url.trim_end_matches('/');
    if base.contains("11434") {
        format!("{base}/api/tags")
    } else {
        format!("{base}/models")
    }
}

fn provider_headers(provider_id: &str, provider: &ProviderCreds) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    let Ok(key) = provider.resolved_key() else {
        return headers;
    };

    if provider_id == "anthropic" {
        if let Ok(value) = reqwest::header::HeaderValue::from_str(&key) {
            headers.insert("x-api-key", value);
        }
        headers.insert(
            "anthropic-version",
            reqwest::header::HeaderValue::from_static("2023-06-01"),
        );
    } else if provider_id != "ollama" {
        if let Ok(value) = reqwest::header::HeaderValue::from_str(&format!("Bearer {key}")) {
            headers.insert(reqwest::header::AUTHORIZATION, value);
        }
    }

    headers
}

// ── DTO builders ──────────────────────────────────────────────────────────────

fn provider_rows(cfg: &Config) -> Vec<ProviderDto> {
    [
        ("anthropic", "Anthropic", cfg.providers.anthropic.as_ref()),
        (
            "openai",
            "OpenAI / OpenRouter",
            cfg.providers.openai.as_ref(),
        ),
        ("ollama", "Ollama", cfg.providers.ollama.as_ref()),
    ]
    .into_iter()
    .map(|(id, name, creds)| provider_row(id, name, creds))
    .collect()
}

fn provider_row(id: &str, name: &str, creds: Option<&ProviderCreds>) -> ProviderDto {
    let (base_url, key_env, timeout, enabled) = if let Some(creds) = creds {
        (
            creds.base_url.clone(),
            creds
                .api_key_env
                .clone()
                .or_else(|| creds.api_key.as_ref().map(|_| "literal key".to_string()))
                .unwrap_or_else(|| "-".to_string()),
            creds.request_timeout_secs,
            true,
        )
    } else {
        ("-".to_string(), "-".to_string(), 0, false)
    };

    ProviderDto {
        id: id.to_string(),
        name: name.to_string(),
        base_url,
        key_env,
        timeout,
        enabled,
        status: if enabled {
            "configured"
        } else {
            "not-configured"
        }
        .to_string(),
    }
}

fn model_rows(cfg: &Config) -> Vec<ModelDto> {
    cfg.models
        .iter()
        .map(|(alias, model)| ModelDto {
            id: alias.clone(),
            alias: alias.clone(),
            provider: model.provider.to_string(),
            model_id: model.model_id.clone(),
            quality: quality_label(model.quality),
            ctx: model.context_window.unwrap_or_default() / 1000,
            tools: model.supports_tools,
            vision: model.supports_vision,
            cost_in: model.input_per_m,
            cost_out: model.output_per_m,
            enabled: model.enabled,
            local: model.local || model.provider == ProviderKind::Ollama,
        })
        .collect()
}

fn rule_rows(cfg: &Config) -> Vec<RuleDto> {
    cfg.rules
        .iter()
        .enumerate()
        .map(|(idx, rule)| RuleDto {
            id: format!("{idx}"),
            name: rule.name.clone(),
            predicate: predicate_label(&rule.when),
            target: target_label(&rule.then.primary, &rule.then.cascade),
        })
        .collect()
}

async fn ledger_rows(cfg: &Config, path: &Path) -> Result<Vec<LedgerRowDto>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let url = format!("sqlite://{}", path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .map_err(|e| e.to_string())?;

    let rows: Vec<LedgerDbRow> = sqlx::query_as(
        "SELECT id, ts, requested_model, route_rule, primary_model, final_model, cascade_path, \
         input_tokens, output_tokens, cost_usd, latency_ms, status \
         FROM requests ORDER BY ts DESC LIMIT 200",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| ledger_row_dto(cfg, row))
        .collect())
}

fn ledger_row_dto(cfg: &Config, row: LedgerDbRow) -> LedgerRowDto {
    let mut cascade = cascade_attempts(cfg, &row.cascade_path);
    if cascade.is_empty() {
        cascade.push(cascade_fallback(cfg, &row.primary_model));
    }
    let status = if row.status == "ok" && cascade.len() > 1 {
        "escalated".to_string()
    } else if row.status == "ok" {
        "ok".to_string()
    } else {
        "error".to_string()
    };

    LedgerRowDto {
        id: row.id,
        ts: local_time_label(row.ts),
        requested: empty_to_placeholder(row.requested_model),
        final_model: row.final_model,
        rule: row.route_rule.unwrap_or_else(|| "policy".to_string()),
        status,
        input_tokens: row.input_tokens,
        output_tokens: row.output_tokens,
        cost: row.cost_usd,
        latency: row.latency_ms,
        cascade,
    }
}

fn cascade_fallback(cfg: &Config, alias: &str) -> CascadeAttemptDto {
    let model = cfg.models.get(alias);
    CascadeAttemptDto {
        attempt: 1,
        alias: alias.to_string(),
        provider: model
            .map(|m| m.provider.to_string())
            .unwrap_or_else(|| "-".to_string()),
        model_id: model
            .map(|m| m.model_id.clone())
            .unwrap_or_else(|| "-".to_string()),
        latency: 0,
        outcome: "recorded".to_string(),
    }
}

fn cascade_attempts(cfg: &Config, raw: &str) -> Vec<CascadeAttemptDto> {
    let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(raw) else {
        return Vec::new();
    };

    values
        .iter()
        .enumerate()
        .map(|(idx, value)| {
            let alias = value
                .get("alias")
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string();
            let model = cfg.models.get(&alias);
            CascadeAttemptDto {
                attempt: idx + 1,
                alias: alias.clone(),
                provider: model
                    .map(|m| m.provider.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                model_id: value
                    .get("model_id")
                    .and_then(|v| v.as_str())
                    .or_else(|| model.map(|m| m.model_id.as_str()))
                    .unwrap_or("-")
                    .to_string(),
                latency: value
                    .get("latency_ms")
                    .and_then(|v| v.as_i64())
                    .unwrap_or_default(),
                outcome: outcome_label(value.get("outcome")),
            }
        })
        .collect()
}

// ── Label helpers ─────────────────────────────────────────────────────────────

fn outcome_label(outcome: Option<&serde_json::Value>) -> String {
    let Some(outcome) = outcome else {
        return "unknown".to_string();
    };
    if let Some(kind) = outcome.get("kind").and_then(|v| v.as_str()) {
        return kind.replace('_', " ");
    }
    if let Some(obj) = outcome.as_object() {
        return obj
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
    }
    "unknown".to_string()
}

fn local_time_label(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|dt: DateTime<Local>| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn empty_to_placeholder(value: String) -> String {
    if value.trim().is_empty() {
        "(none)".to_string()
    } else {
        value
    }
}

fn predicate_label(predicate: &Predicate) -> String {
    let mut parts = Vec::new();
    if predicate.always == Some(true) {
        parts.push("always".to_string());
    }
    if let Some(value) = predicate.has_tools {
        parts.push(format!("has_tools = {value}"));
    }
    if let Some(value) = predicate.tokens_in_gt {
        parts.push(format!("tokens_in_gt {value}"));
    }
    if let Some(value) = predicate.tokens_in_lt {
        parts.push(format!("tokens_in_lt {value}"));
    }
    if let Some(value) = &predicate.system_matches {
        parts.push(format!("system_matches {value}"));
    }
    if let Some(value) = &predicate.model_alias_eq {
        parts.push(format!("model_alias_eq {value}"));
    }
    if let Some(header) = &predicate.header {
        let suffix = header
            .equals
            .as_ref()
            .map(|v| format!(" = {v}"))
            .unwrap_or_else(|| {
                if header.any_value {
                    " any".to_string()
                } else {
                    String::new()
                }
            });
        parts.push(format!("header {}{}", header.name, suffix));
    }
    if parts.is_empty() {
        "empty".to_string()
    } else {
        parts.join(", ")
    }
}

fn target_label(primary: &str, cascade: &[String]) -> String {
    std::iter::once(primary.to_string())
        .chain(cascade.iter().cloned())
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn routing_profile_label(profile: RoutingProfile) -> String {
    match profile {
        RoutingProfile::LocalFirst => "local-first",
        RoutingProfile::Balanced => "balanced",
        RoutingProfile::BestQuality => "best-quality",
        RoutingProfile::CloudOnly => "cloud-only",
    }
    .to_string()
}

fn quality_label(quality: ModelQuality) -> String {
    match quality {
        ModelQuality::Basic => "basic",
        ModelQuality::Standard => "standard",
        ModelQuality::Strong => "strong",
        ModelQuality::Premium => "premium",
    }
    .to_string()
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn mode_label() -> String {
    if cfg!(windows) {
        "Windows desktop".to_string()
    } else if cfg!(target_os = "macos") {
        "macOS desktop".to_string()
    } else {
        "Linux desktop".to_string()
    }
}

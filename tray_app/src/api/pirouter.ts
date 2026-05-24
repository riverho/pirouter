import { invoke } from "@tauri-apps/api/core";

export type DaemonStatus = "running" | "stopped" | "error" | "restart-required";
export type LedgerStatus = "ok" | "error" | "escalated";

export interface DaemonInfo {
  status: DaemonStatus;
  mode: string;
  endpoint: string;
  healthUrl: string;
  bind: string;
  configPath: string;
  ledgerPath: string;
}

export interface RoutingInfo {
  profile: string;
  autoCascade: boolean;
  maxFallbacks: number;
  onHttpError: boolean;
  onShortResponse: boolean;
  minOutputTokens: number;
  marker: string;
}

export interface ProviderInfo {
  id: string;
  name: string;
  baseUrl: string;
  keyEnv: string;
  timeout: number;
  enabled: boolean;
  status: "configured" | "healthy" | "error" | "not-configured" | "testing";
}

export interface ModelInfo {
  id: string;
  alias: string;
  provider: string;
  modelId: string;
  quality: string;
  ctx: number;
  tools: boolean;
  vision: boolean;
  costIn: number;
  costOut: number;
  enabled: boolean;
  local: boolean;
}

export interface RuleInfo {
  id: string;
  name: string;
  predicate: string;
  target: string;
}

export interface CascadeAttempt {
  attempt: number;
  alias: string;
  provider: string;
  modelId: string;
  latency: number;
  outcome: string;
}

export interface LedgerRow {
  id: string;
  ts: string;
  requested: string;
  finalModel: string;
  rule: string;
  status: LedgerStatus;
  inputTokens: number;
  outputTokens: number;
  cost: number;
  latency: number;
  cascade: CascadeAttempt[];
}

export interface AppState {
  daemon: DaemonInfo;
  routing: RoutingInfo;
  providers: ProviderInfo[];
  models: ModelInfo[];
  rules: RuleInfo[];
  ledger: LedgerRow[];
}

const isTauri = () => "__TAURI_INTERNALS__" in window;

export const loadAppState = async (): Promise<AppState> => {
  if (!isTauri()) return mockAppState;
  return invoke<AppState>("load_app_state");
};

export const runDaemonAction = async (action: "start" | "stop" | "restart") => {
  if (!isTauri()) return `mock ${action}`;
  return invoke<string>("daemon_action", { action });
};

export const validateConfig = async () => {
  if (!isTauri()) return "Config is valid: browser preview";
  return invoke<string>("validate_config");
};

export const testProvider = async (providerId: string) => {
  if (!isTauri()) return `${providerId} reachable in browser preview`;
  return invoke<string>("test_provider", { providerId });
};

export const quitApp = async () => {
  if (!isTauri()) return;
  return invoke<void>("quit_app");
};

const mockAppState: AppState = {
  daemon: {
    status: "running",
    mode: "Browser preview",
    endpoint: "http://127.0.0.1:11435/v1",
    healthUrl: "http://127.0.0.1:11435/healthz",
    bind: "127.0.0.1:11435",
    configPath: "C:\\Users\\RH\\AppData\\Roaming\\pirouter\\config\\config.toml",
    ledgerPath: "C:\\Users\\RH\\AppData\\Roaming\\pirouter\\data\\ledger.db",
  },
  routing: {
    profile: "balanced",
    autoCascade: true,
    maxFallbacks: 3,
    onHttpError: true,
    onShortResponse: false,
    minOutputTokens: 8,
    marker: "",
  },
  providers: [
    {
      id: "openai",
      name: "OpenAI-compatible",
      baseUrl: "https://openrouter.ai/api/v1",
      keyEnv: "OPENROUTER_API_KEY",
      timeout: 120,
      enabled: true,
      status: "configured",
    },
    {
      id: "ollama",
      name: "Ollama",
      baseUrl: "http://127.0.0.1:11434",
      keyEnv: "-",
      timeout: 120,
      enabled: false,
      status: "not-configured",
    },
  ],
  models: [
    {
      id: "deepseek-flash",
      alias: "deepseek-flash",
      provider: "openai",
      modelId: "deepseek/deepseek-chat-v3-0324",
      quality: "standard",
      ctx: 128,
      tools: false,
      vision: false,
      costIn: 0.28,
      costOut: 0.88,
      enabled: true,
      local: false,
    },
    {
      id: "glm5",
      alias: "glm5",
      provider: "openai",
      modelId: "z-ai/glm-4.5",
      quality: "strong",
      ctx: 128,
      tools: true,
      vision: false,
      costIn: 0.6,
      costOut: 2.2,
      enabled: true,
      local: false,
    },
  ],
  rules: [
    {
      id: "0",
      name: "tools-thinking",
      predicate: "has_tools = true",
      target: "glm5 -> qwen-thinking -> deepseek-flash",
    },
    {
      id: "1",
      name: "default",
      predicate: "always",
      target: "deepseek-flash -> qwen-flash -> qwen-thinking",
    },
  ],
  ledger: [],
};

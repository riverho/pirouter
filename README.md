# pirouter

```bash
This app is still undergoing development, please note.
```

`pirouter` is a lightweight LLM routing daemon. It speaks the OpenAI Chat
Completions API to your agents and routes each request to the cheapest model
that is likely to handle it well.

Point tools like Aider, Continue, OpenWebUI, LangChain clients, or your own
scripts at one local OpenAI-compatible endpoint. pirouter decides whether the
work should go to a local Ollama model, a cheaper cloud model, or a stronger
fallback.

## Why pirouter

- **One local endpoint for many providers.** OpenAI-compatible clients only
  need to change `base_url`.
- **Cost-aware routing.** Route easy requests to cheaper/local models and save
  stronger models for tasks that need them.
- **Declarative rules.** Keep routing policy in TOML instead of scattering it
  through prompts and agent code.
- **Capability-aware defaults.** If no rule matches, pirouter can choose from
  the model catalog by quality tier, tool support, context window, local/cloud
  placement, and routing profile.
- **Cascade fallback.** Escalate from a cheap primary model to stronger models
  on provider errors, short responses, or explicit escalation markers.
- **Observable spend.** A SQLite ledger records model attempts, tokens, cost,
  latency, and escalation path.
- **Local-first friendly.** Ollama is supported as a first-class provider.

## Install

### From GitHub

```bash
cargo install --git https://github.com/riverho/pirouter --tag v0.1.0
```

### From a local checkout

```bash
git clone https://github.com/riverho/pirouter.git
cd pirouter
cargo install --path .
```

## Configure

Create a config directory and start from the example config:

```bash
mkdir -p ~/.config/pirouter
cp config.example.toml ~/.config/pirouter/config.toml
```

On Windows PowerShell:

```powershell
New-Item -ItemType Directory -Force $env:APPDATA\pirouter
Copy-Item config.example.toml $env:APPDATA\pirouter\config.toml
```

Set provider credentials in your environment. Ollama does not need an API key.

```bash
export ANTHROPIC_API_KEY="..."
export OPENAI_API_KEY="..."
```

PowerShell:

```powershell
$env:ANTHROPIC_API_KEY = "..."
$env:OPENAI_API_KEY = "..."
```

For local models, install Ollama and pull a model referenced by your config:

```bash
ollama pull llama3.2:3b
ollama pull qwen2.5-coder:7b
```

## Run

Validate config:

```bash
pirouter check-config
pirouter models
pirouter route --prompt "debug this SQL migration"
```

Start the daemon:

```bash
pirouter run
```

By default pirouter listens on:

```text
http://127.0.0.1:11435/v1
```

Send an OpenAI-compatible request:

```bash
curl http://127.0.0.1:11435/v1/chat/completions \
  -H "content-type: application/json" \
  -d '{
    "model": "auto",
    "messages": [{"role":"user","content":"Hello from pirouter"}]
  }'
```

## Routing

Rules are evaluated top-to-bottom. The first match wins.

```toml
[[rules]]
name = "explicit-override"
when = { header = { name = "x-pirouter-route", any_value = true } }
then = { primary = "$header:x-pirouter-route", cascade = [] }

[routing]
profile = "balanced" # local-first | balanced | best-quality | cloud-only
auto_cascade = true
max_policy_fallbacks = 3
```

Clients can also send a simple difficulty hint:

```text
x-pirouter-difficulty: easy | standard | hard
```

## Inspect Spend

```bash
pirouter stats --hours 24
```

The ledger is stored in the platform data directory unless overridden in
`config.toml`. It is plain SQLite, so you can inspect it with normal SQLite
tools.

## Status

`v0.1.0` includes:

- OpenAI-compatible `/v1/chat/completions`
- Anthropic, OpenAI, and Ollama provider adapters
- TOML routing rules
- Capability-aware policy router
- Cascade fallback
- SQLite ledger and `pirouter stats`
- CLI config validation, model listing, and route dry runs

Planned next surfaces include true streaming, a tray app, semantic local
routing, MCP control tools, and optional Pi agent integration.

## License

Apache-2.0. See [LICENSE](LICENSE).

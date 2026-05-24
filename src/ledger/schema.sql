-- pirouter cost ledger. One row per request, even if the cascade touched
-- multiple models — the per-attempt detail lives in `cascade_path` JSON.

CREATE TABLE IF NOT EXISTS requests (
    id            TEXT PRIMARY KEY,            -- ULID
    ts            INTEGER NOT NULL,            -- unix seconds
    requested_model TEXT NOT NULL DEFAULT '',  -- alias/model the client asked for
    route_rule    TEXT,                        -- which rule matched
    primary_model TEXT NOT NULL,               -- first model attempted
    final_model   TEXT NOT NULL,               -- final alias that answered
    cascade_path  TEXT NOT NULL,               -- JSON array of CascadeAttempt
    input_tokens  INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd      REAL NOT NULL DEFAULT 0.0,
    latency_ms    INTEGER NOT NULL DEFAULT 0,
    status        TEXT NOT NULL                -- 'ok' | 'error' | 'exhausted'
);

CREATE INDEX IF NOT EXISTS idx_requests_ts ON requests(ts);
CREATE INDEX IF NOT EXISTS idx_requests_final_model ON requests(final_model);

CREATE TABLE IF NOT EXISTS _meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS models (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    architecture TEXT,
    quantization TEXT,
    context_metadata INTEGER,
    supports_chat INTEGER NOT NULL DEFAULT 0,
    supports_streaming INTEGER NOT NULL DEFAULT 0,
    supports_tools INTEGER NOT NULL DEFAULT 0,
    supports_vision INTEGER NOT NULL DEFAULT 0,
    supports_embeddings INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    last_verified TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runtime_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    model_id TEXT,
    backend TEXT NOT NULL,
    context_size INTEGER NOT NULL,
    threads INTEGER NOT NULL,
    batch_size INTEGER NOT NULL,
    gpu_layers INTEGER NOT NULL,
    gpu_offload INTEGER NOT NULL,
    port INTEGER NOT NULL,
    host TEXT NOT NULL,
    extra_args TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS integration_state (
    app TEXT PRIMARY KEY,
    detected_version TEXT,
    status TEXT NOT NULL,
    protocol TEXT,
    last_verified TEXT,
    notes TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS config_transactions (
    id TEXT PRIMARY KEY,
    app TEXT NOT NULL,
    config_path TEXT NOT NULL,
    action TEXT NOT NULL,
    original_sha256 TEXT NOT NULL,
    resulting_sha256 TEXT,
    backup_path TEXT,
    committed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT NOT NULL,
    detail TEXT,
    result TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_config_transactions_app ON config_transactions(app);
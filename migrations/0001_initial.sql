-- 基础配置表：租户

CREATE TABLE IF NOT EXISTS tenants (
    id          TEXT PRIMARY KEY,            -- UUID
    name        TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active', -- active | suspended
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- API 密钥表

CREATE TABLE IF NOT EXISTS api_keys (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL REFERENCES tenants(id),
    hashed_key      TEXT NOT NULL UNIQUE,
    name            TEXT,
    status          TEXT NOT NULL DEFAULT 'active',  -- active | revoked
    quota_rpm       INTEGER,   -- 每分钟请求限额，NULL 表示不限
    quota_tpm       INTEGER,   -- 每分钟 Token 限额，NULL 表示不限
    quota_daily_req INTEGER,   -- 每日请求限额
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- 提供商表

CREATE TABLE IF NOT EXISTS providers (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,   -- openai | anthropic | copilot | ...
    display_name    TEXT NOT NULL,
    base_url        TEXT NOT NULL,
    auth_mode       TEXT NOT NULL DEFAULT 'bearer',  -- bearer | api-key-header
    auth_header     TEXT,                            -- 自定义 header 名
    enabled         INTEGER NOT NULL DEFAULT 1,      -- 0 | 1
    health_state    TEXT NOT NULL DEFAULT 'unknown', -- healthy | degraded | offline | unknown
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- 提供商密钥池

CREATE TABLE IF NOT EXISTS provider_keys (
    id          TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id),
    key_ref     TEXT NOT NULL,   -- 加密存储的 key
    label       TEXT,
    enabled     INTEGER NOT NULL DEFAULT 1,
    priority    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL
);

-- 提供商模型

CREATE TABLE IF NOT EXISTS provider_models (
    id                  TEXT PRIMARY KEY,
    provider_id         TEXT NOT NULL REFERENCES providers(id),
    model_name          TEXT NOT NULL,
    enabled             INTEGER NOT NULL DEFAULT 1,
    supports_vision     INTEGER NOT NULL DEFAULT 0,
    supports_streaming  INTEGER NOT NULL DEFAULT 1,
    supports_tools      INTEGER NOT NULL DEFAULT 1,
    context_window      INTEGER,
    max_output_tokens   INTEGER,
    cost_input_per_1k   REAL,
    cost_output_per_1k  REAL,
    outbound_proxy_id   TEXT REFERENCES outbound_proxies(id),
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    UNIQUE(provider_id, model_name)
);

-- 出网代理配置

CREATE TABLE IF NOT EXISTS outbound_proxies (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    scheme      TEXT NOT NULL DEFAULT 'socks5',  -- http | https | socks5
    host        TEXT NOT NULL,
    port        INTEGER NOT NULL,
    username    TEXT,
    password    TEXT,  -- 加密存储
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- 逻辑模型别名

CREATE TABLE IF NOT EXISTS model_aliases (
    id              TEXT PRIMARY KEY,
    alias_name      TEXT NOT NULL UNIQUE,
    description     TEXT,
    route_strategy  TEXT NOT NULL DEFAULT 'priority',  -- priority | latency | cost
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- 别名目标（有序）

CREATE TABLE IF NOT EXISTS model_alias_targets (
    id              TEXT PRIMARY KEY,
    alias_id        TEXT NOT NULL REFERENCES model_aliases(id),
    provider_id     TEXT NOT NULL REFERENCES providers(id),
    model_name      TEXT NOT NULL,
    priority        INTEGER NOT NULL DEFAULT 0,  -- 数字越小优先级越高
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL
);

-- 回退触发规则

CREATE TABLE IF NOT EXISTS failover_rules (
    id          TEXT PRIMARY KEY,
    alias_id    TEXT NOT NULL REFERENCES model_aliases(id),
    trigger     TEXT NOT NULL,  -- insufficient_balance | rate_limited | upstream_5xx | timeout | model_offline | manual_disabled
    enabled     INTEGER NOT NULL DEFAULT 1
);

-- 请求日志

CREATE TABLE IF NOT EXISTS request_logs (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    api_key_id      TEXT NOT NULL,
    request_id      TEXT NOT NULL UNIQUE,
    model_alias     TEXT NOT NULL,
    provider_id     TEXT,
    provider_model  TEXT,
    origin_protocol TEXT NOT NULL,
    status          TEXT NOT NULL,  -- success | error | failover
    input_tokens    INTEGER,
    output_tokens   INTEGER,
    latency_ms      INTEGER,
    failover_count  INTEGER NOT NULL DEFAULT 0,
    error_code      TEXT,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_request_logs_tenant ON request_logs(tenant_id, created_at);
CREATE INDEX IF NOT EXISTS idx_request_logs_key ON request_logs(api_key_id, created_at);

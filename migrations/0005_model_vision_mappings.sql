CREATE TABLE IF NOT EXISTS model_vision_mappings (
    id                  TEXT PRIMARY KEY,
    model_name          TEXT NOT NULL UNIQUE,
    vision_parser_alias TEXT,
    generation_alias    TEXT,
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_model_vision_mappings_model ON model_vision_mappings(model_name);

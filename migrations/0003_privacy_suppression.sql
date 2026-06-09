CREATE TABLE privacy_suppressions (
    id UUID PRIMARY KEY,
    scope_type TEXT NOT NULL,
    scope_value TEXT,
    suppression_hash TEXT NOT NULL,
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    UNIQUE (scope_type, scope_value, suppression_hash)
);

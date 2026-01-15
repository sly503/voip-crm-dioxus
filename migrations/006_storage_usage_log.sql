-- Storage Usage Log Migration
-- Table for tracking daily storage usage statistics and metrics

CREATE TABLE storage_usage_log (
    id BIGSERIAL PRIMARY KEY,
    date DATE NOT NULL UNIQUE,
    total_files BIGINT NOT NULL DEFAULT 0,
    total_size_bytes BIGINT NOT NULL DEFAULT 0,
    recordings_added INT NOT NULL DEFAULT 0,
    recordings_deleted INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for better query performance
CREATE INDEX idx_storage_usage_log_date ON storage_usage_log(date DESC);
CREATE INDEX idx_storage_usage_log_created_at ON storage_usage_log(created_at);

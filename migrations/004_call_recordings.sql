-- Call Recordings Migration
-- Table for storing call recording metadata and file information

CREATE TABLE call_recordings (
    id BIGSERIAL PRIMARY KEY,
    call_id BIGINT NOT NULL REFERENCES calls(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    duration_seconds INT NOT NULL,
    format VARCHAR(10) NOT NULL DEFAULT 'wav',
    encryption_key_id VARCHAR(255) NOT NULL,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retention_until TIMESTAMPTZ NOT NULL,
    compliance_hold BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for better query performance
CREATE INDEX idx_call_recordings_call_id ON call_recordings(call_id);
CREATE INDEX idx_call_recordings_uploaded_at ON call_recordings(uploaded_at);
CREATE INDEX idx_call_recordings_retention_until ON call_recordings(retention_until);
CREATE INDEX idx_call_recordings_compliance_hold ON call_recordings(compliance_hold) WHERE compliance_hold = TRUE;
CREATE INDEX idx_call_recordings_metadata ON call_recordings USING gin(metadata);

-- Recording Audit Log Migration
-- Table and triggers for tracking all recording access and modifications

-- Create ENUM type for audit actions
CREATE TYPE recording_audit_action AS ENUM (
    'uploaded',
    'downloaded',
    'deleted',
    'hold_set',
    'hold_released'
);

-- Create audit log table
CREATE TABLE recording_audit_log (
    id BIGSERIAL PRIMARY KEY,
    recording_id BIGINT NOT NULL,  -- Not FK to allow audit trail after deletion
    action recording_audit_action NOT NULL,
    user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,  -- NULL for system actions
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address INET,  -- NULL for system-triggered actions
    metadata JSONB  -- Additional context (e.g., file_path, size for deleted recordings)
);

-- Indexes for better query performance
CREATE INDEX idx_recording_audit_log_recording_id ON recording_audit_log(recording_id);
CREATE INDEX idx_recording_audit_log_timestamp ON recording_audit_log(timestamp DESC);
CREATE INDEX idx_recording_audit_log_user_id ON recording_audit_log(user_id);
CREATE INDEX idx_recording_audit_log_action ON recording_audit_log(action);

-- Function to log recording upload
CREATE OR REPLACE FUNCTION log_recording_upload()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO recording_audit_log (recording_id, action, metadata)
    VALUES (
        NEW.id,
        'uploaded',
        jsonb_build_object(
            'file_path', NEW.file_path,
            'file_size', NEW.file_size,
            'duration_seconds', NEW.duration_seconds,
            'format', NEW.format
        )
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function to log recording deletion
CREATE OR REPLACE FUNCTION log_recording_deletion()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO recording_audit_log (recording_id, action, metadata)
    VALUES (
        OLD.id,
        'deleted',
        jsonb_build_object(
            'file_path', OLD.file_path,
            'file_size', OLD.file_size,
            'duration_seconds', OLD.duration_seconds,
            'compliance_hold', OLD.compliance_hold,
            'retention_until', OLD.retention_until
        )
    );
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

-- Function to log compliance hold changes
CREATE OR REPLACE FUNCTION log_compliance_hold_change()
RETURNS TRIGGER AS $$
BEGIN
    -- Only log if compliance_hold actually changed
    IF OLD.compliance_hold != NEW.compliance_hold THEN
        INSERT INTO recording_audit_log (recording_id, action, metadata)
        VALUES (
            NEW.id,
            CASE WHEN NEW.compliance_hold THEN 'hold_set' ELSE 'hold_released' END,
            jsonb_build_object(
                'file_path', NEW.file_path,
                'previous_hold', OLD.compliance_hold,
                'new_hold', NEW.compliance_hold
            )
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for recording upload
CREATE TRIGGER trigger_log_recording_upload
    AFTER INSERT ON call_recordings
    FOR EACH ROW
    EXECUTE FUNCTION log_recording_upload();

-- Trigger for recording deletion
CREATE TRIGGER trigger_log_recording_deletion
    BEFORE DELETE ON call_recordings
    FOR EACH ROW
    EXECUTE FUNCTION log_recording_deletion();

-- Trigger for compliance hold changes
CREATE TRIGGER trigger_log_compliance_hold_change
    AFTER UPDATE ON call_recordings
    FOR EACH ROW
    WHEN (OLD.compliance_hold IS DISTINCT FROM NEW.compliance_hold)
    EXECUTE FUNCTION log_compliance_hold_change();

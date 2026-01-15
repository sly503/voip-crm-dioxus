-- Recording Retention Policies Migration
-- Table for managing automatic deletion policies for call recordings

-- Custom enum for policy scope
CREATE TYPE retention_applies_to AS ENUM ('All', 'Campaign', 'Agent');

CREATE TABLE recording_retention_policies (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    retention_days INT NOT NULL,
    applies_to retention_applies_to NOT NULL DEFAULT 'All',
    campaign_id BIGINT REFERENCES campaigns(id) ON DELETE CASCADE,
    agent_id BIGINT REFERENCES agents(id) ON DELETE CASCADE,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for better query performance
CREATE INDEX idx_retention_policies_applies_to ON recording_retention_policies(applies_to);
CREATE INDEX idx_retention_policies_campaign_id ON recording_retention_policies(campaign_id) WHERE campaign_id IS NOT NULL;
CREATE INDEX idx_retention_policies_agent_id ON recording_retention_policies(agent_id) WHERE agent_id IS NOT NULL;
CREATE INDEX idx_retention_policies_is_default ON recording_retention_policies(is_default) WHERE is_default = TRUE;

-- Constraints to ensure data integrity
-- Campaign-specific policies must have a campaign_id
ALTER TABLE recording_retention_policies
ADD CONSTRAINT check_campaign_policy CHECK (
    (applies_to = 'Campaign' AND campaign_id IS NOT NULL) OR applies_to != 'Campaign'
);

-- Agent-specific policies must have an agent_id
ALTER TABLE recording_retention_policies
ADD CONSTRAINT check_agent_policy CHECK (
    (applies_to = 'Agent' AND agent_id IS NOT NULL) OR applies_to != 'Agent'
);

-- All-scope policies should not have campaign_id or agent_id
ALTER TABLE recording_retention_policies
ADD CONSTRAINT check_all_policy CHECK (
    (applies_to = 'All' AND campaign_id IS NULL AND agent_id IS NULL) OR applies_to != 'All'
);

-- Ensure only one default policy exists
CREATE UNIQUE INDEX idx_unique_default_policy ON recording_retention_policies(is_default) WHERE is_default = TRUE;

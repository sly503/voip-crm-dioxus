-- Add Recording Fields to Campaigns
-- Adds consent announcement and recording enabled fields for call recording configuration

ALTER TABLE campaigns
ADD COLUMN consent_announcement TEXT,
ADD COLUMN recording_enabled BOOLEAN NOT NULL DEFAULT true;

-- Add comment to explain the fields
COMMENT ON COLUMN campaigns.consent_announcement IS 'Audio message played to customers before recording starts';
COMMENT ON COLUMN campaigns.recording_enabled IS 'Whether call recording is enabled for this campaign';

-- AI Enhancements Migration

-- Add unique constraint on agent_id for upsert operations
ALTER TABLE ai_agent_settings
ADD CONSTRAINT unique_agent_ai_settings UNIQUE (agent_id);

-- Global AI configuration table
CREATE TABLE global_ai_config (
    id BIGSERIAL PRIMARY KEY,
    key VARCHAR(255) NOT NULL UNIQUE,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- AI conversation history for calls
CREATE TABLE ai_conversations (
    id BIGSERIAL PRIMARY KEY,
    call_id BIGINT NOT NULL REFERENCES calls(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL, -- 'user', 'assistant', 'system'
    content TEXT NOT NULL,
    tokens_used INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Prompt templates table
CREATE TABLE prompt_templates (
    id VARCHAR(100) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    category VARCHAR(100) NOT NULL,
    content TEXT NOT NULL,
    variables TEXT[], -- Array of variable names like 'lead.name', 'campaign.name'
    is_default BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Campaign automation state
CREATE TABLE campaign_automation (
    id BIGSERIAL PRIMARY KEY,
    campaign_id BIGINT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE UNIQUE,
    is_running BOOLEAN NOT NULL DEFAULT FALSE,
    last_dial_at TIMESTAMPTZ,
    current_lead_index INT DEFAULT 0,
    calls_in_progress INT DEFAULT 0,
    error_message TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Scheduled callbacks
CREATE TABLE scheduled_callbacks (
    id BIGSERIAL PRIMARY KEY,
    lead_id BIGINT NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
    agent_id BIGINT REFERENCES agents(id),
    scheduled_at TIMESTAMPTZ NOT NULL,
    notes TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- pending, completed, cancelled
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_ai_conversations_call ON ai_conversations(call_id);
CREATE INDEX idx_prompt_templates_category ON prompt_templates(category);
CREATE INDEX idx_scheduled_callbacks_time ON scheduled_callbacks(scheduled_at) WHERE status = 'pending';
CREATE INDEX idx_campaign_automation_running ON campaign_automation(is_running) WHERE is_running = TRUE;

-- Insert default prompt templates
INSERT INTO prompt_templates (id, name, category, content, variables, is_default) VALUES
('sales-intro', 'Sales Introduction', 'Sales',
 'You are a professional sales representative for {{company_name}}. Your goal is to introduce our products/services to {{lead.name}} and qualify their interest. Be friendly, professional, and listen carefully to their needs.',
 ARRAY['company_name', 'lead.name'], TRUE),

('followup-call', 'Follow-up Call', 'Follow-up',
 'You are calling {{lead.name}} as a follow-up to our previous conversation. Reference any notes from prior calls and continue building the relationship. Ask about any questions they may have had since we last spoke.',
 ARRAY['lead.name'], TRUE),

('support-call', 'Customer Support', 'Support',
 'You are a customer support representative helping {{lead.name}} with their inquiry. Be patient, empathetic, and thorough in addressing their concerns. If you cannot resolve an issue, offer to transfer to a human agent.',
 ARRAY['lead.name'], TRUE),

('survey-call', 'Customer Survey', 'Survey',
 'You are conducting a brief customer satisfaction survey with {{lead.name}}. Ask about their experience with our product/service and gather feedback. Thank them for their time and any suggestions they provide.',
 ARRAY['lead.name'], TRUE);

-- Insert default global AI config
INSERT INTO global_ai_config (key, value) VALUES
('model', 'claude-sonnet-4-5-20250514'),
('use_claude_code', 'true'),
('fallback_to_api', 'true'),
('default_voice', 'alloy'),
('max_call_duration', '300'),
('stt_provider', 'deepgram'),
('tts_provider', 'openai');

-- VoIP CRM Initial Schema

-- Custom enum types
CREATE TYPE user_role AS ENUM ('Admin', 'Supervisor', 'Agent');
CREATE TYPE agent_type AS ENUM ('Human', 'Ai');
CREATE TYPE agent_status AS ENUM ('Offline', 'Ready', 'OnCall', 'AfterCall', 'Break');
CREATE TYPE call_status AS ENUM ('Initiated', 'Ringing', 'Answered', 'Bridged', 'Completed', 'NoAnswer', 'Busy', 'Failed');
CREATE TYPE call_direction AS ENUM ('Inbound', 'Outbound');
CREATE TYPE lead_status AS ENUM ('New', 'Contacted', 'Qualified', 'Converted', 'Lost', 'DoNotCall');
CREATE TYPE campaign_status AS ENUM ('Draft', 'Active', 'Paused', 'Completed');
CREATE TYPE dialer_mode AS ENUM ('Preview', 'Progressive', 'Predictive');

-- Users table
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    role user_role NOT NULL DEFAULT 'Agent',
    first_name VARCHAR(255),
    last_name VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Agents table
CREATE TABLE agents (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    extension VARCHAR(50),
    user_id BIGINT REFERENCES users(id),
    agent_type agent_type NOT NULL DEFAULT 'Human',
    status agent_status NOT NULL DEFAULT 'Offline',
    sip_username VARCHAR(255),
    current_call_id BIGINT,
    last_status_change TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Campaigns table
CREATE TABLE campaigns (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    status campaign_status NOT NULL DEFAULT 'Draft',
    dialer_mode dialer_mode NOT NULL DEFAULT 'Preview',
    caller_id VARCHAR(50),
    start_time TIME,
    end_time TIME,
    max_attempts INT NOT NULL DEFAULT 3,
    retry_delay_minutes INT NOT NULL DEFAULT 30,
    total_leads INT DEFAULT 0,
    dialed_leads INT DEFAULT 0,
    connected_leads INT DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Leads table
CREATE TABLE leads (
    id BIGSERIAL PRIMARY KEY,
    first_name VARCHAR(255),
    last_name VARCHAR(255),
    phone VARCHAR(50) NOT NULL,
    email VARCHAR(255),
    company VARCHAR(255),
    status lead_status NOT NULL DEFAULT 'New',
    notes TEXT,
    campaign_id BIGINT REFERENCES campaigns(id),
    assigned_agent_id BIGINT REFERENCES agents(id),
    call_attempts INT NOT NULL DEFAULT 0,
    last_call_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Calls table
CREATE TABLE calls (
    id BIGSERIAL PRIMARY KEY,
    call_control_id VARCHAR(255) UNIQUE,
    lead_id BIGINT REFERENCES leads(id),
    agent_id BIGINT REFERENCES agents(id),
    campaign_id BIGINT REFERENCES campaigns(id),
    direction call_direction NOT NULL DEFAULT 'Outbound',
    status call_status NOT NULL DEFAULT 'Initiated',
    from_number VARCHAR(50),
    to_number VARCHAR(50),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    answered_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    duration_seconds INT,
    disposition VARCHAR(255),
    recording_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- AI Agent Settings table
CREATE TABLE ai_agent_settings (
    id BIGSERIAL PRIMARY KEY,
    agent_id BIGINT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    system_prompt TEXT NOT NULL,
    greeting_message TEXT,
    voice_id VARCHAR(100),
    language VARCHAR(10) NOT NULL DEFAULT 'en-US',
    max_response_tokens INT DEFAULT 150,
    temperature FLOAT DEFAULT 0.7,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Campaign Agents (many-to-many)
CREATE TABLE campaign_agents (
    campaign_id BIGINT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    agent_id BIGINT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    PRIMARY KEY (campaign_id, agent_id)
);

-- Call Notes table
CREATE TABLE call_notes (
    id BIGSERIAL PRIMARY KEY,
    call_id BIGINT NOT NULL REFERENCES calls(id) ON DELETE CASCADE,
    agent_id BIGINT REFERENCES agents(id),
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add foreign key for current_call_id after calls table exists
ALTER TABLE agents ADD CONSTRAINT fk_agent_current_call
    FOREIGN KEY (current_call_id) REFERENCES calls(id);

-- Indexes for better query performance
CREATE INDEX idx_leads_status ON leads(status);
CREATE INDEX idx_leads_campaign ON leads(campaign_id);
CREATE INDEX idx_leads_agent ON leads(assigned_agent_id);
CREATE INDEX idx_leads_phone ON leads(phone);

CREATE INDEX idx_calls_status ON calls(status);
CREATE INDEX idx_calls_agent ON calls(agent_id);
CREATE INDEX idx_calls_lead ON calls(lead_id);
CREATE INDEX idx_calls_campaign ON calls(campaign_id);
CREATE INDEX idx_calls_started_at ON calls(started_at);
CREATE INDEX idx_calls_control_id ON calls(call_control_id);

CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_user ON agents(user_id);

CREATE INDEX idx_campaigns_status ON campaigns(status);

-- Insert default admin user (password: admin123)
INSERT INTO users (username, email, password_hash, role, first_name, last_name)
VALUES ('admin', 'admin@voipcrm.local', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/X4.HjLGASQ86BwaBm', 'Admin', 'System', 'Admin');

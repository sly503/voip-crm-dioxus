use crate::api::{api_client, ApiError};
use crate::models::{Agent, CreateAgentRequest, UpdateAgentStatusRequest, AgentStatus};

pub async fn get_all_agents() -> Result<Vec<Agent>, ApiError> {
    api_client().get("/api/agents").await
}

pub async fn create_agent(request: CreateAgentRequest) -> Result<Agent, ApiError> {
    api_client().post("/api/agents", &request).await
}

pub async fn update_agent_status(agent_id: i64, status: AgentStatus) -> Result<Agent, ApiError> {
    let request = UpdateAgentStatusRequest { status };
    api_client().put(&format!("/api/agents/{}/status", agent_id), &request).await
}

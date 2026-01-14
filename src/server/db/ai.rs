//! AI settings database operations

use sqlx::PgPool;
use crate::models::AiAgentSettings;

/// Get AI settings for an agent
pub async fn get_settings(pool: &PgPool, agent_id: i64) -> Result<Option<AiAgentSettings>, sqlx::Error> {
    sqlx::query_as::<_, AiAgentSettings>(
        r"
        SELECT id, agent_id, system_prompt, greeting_message, voice_id,
               language, max_response_tokens, temperature, created_at, updated_at
        FROM ai_agent_settings
        WHERE agent_id = $1
        "
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await
}

/// Get all AI settings
pub async fn get_all_settings(pool: &PgPool) -> Result<Vec<AiAgentSettings>, sqlx::Error> {
    sqlx::query_as::<_, AiAgentSettings>(
        r"
        SELECT id, agent_id, system_prompt, greeting_message, voice_id,
               language, max_response_tokens, temperature, created_at, updated_at
        FROM ai_agent_settings
        ORDER BY agent_id
        "
    )
    .fetch_all(pool)
    .await
}

/// Create or update AI settings for an agent
#[allow(clippy::too_many_arguments)]
pub async fn upsert_settings(
    pool: &PgPool,
    agent_id: i64,
    system_prompt: &str,
    greeting_message: Option<&str>,
    voice_id: Option<&str>,
    language: &str,
    max_response_tokens: Option<i32>,
    temperature: Option<f64>,
) -> Result<AiAgentSettings, sqlx::Error> {
    sqlx::query_as::<_, AiAgentSettings>(
        r"
        INSERT INTO ai_agent_settings (agent_id, system_prompt, greeting_message, voice_id, language, max_response_tokens, temperature, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        ON CONFLICT (agent_id) DO UPDATE SET
            system_prompt = EXCLUDED.system_prompt,
            greeting_message = EXCLUDED.greeting_message,
            voice_id = EXCLUDED.voice_id,
            language = EXCLUDED.language,
            max_response_tokens = EXCLUDED.max_response_tokens,
            temperature = EXCLUDED.temperature,
            updated_at = NOW()
        RETURNING id, agent_id, system_prompt, greeting_message, voice_id,
                  language, max_response_tokens, temperature, created_at, updated_at
        "
    )
    .bind(agent_id)
    .bind(system_prompt)
    .bind(greeting_message)
    .bind(voice_id)
    .bind(language)
    .bind(max_response_tokens)
    .bind(temperature)
    .fetch_one(pool)
    .await
}

/// Delete AI settings for an agent
pub async fn delete_settings(pool: &PgPool, agent_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r"
        DELETE FROM ai_agent_settings
        WHERE agent_id = $1
        "
    )
    .bind(agent_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

// ============== Prompt Templates ==============

use crate::models::PromptTemplate;

/// Get all prompt templates
pub async fn get_all_templates(pool: &PgPool) -> Result<Vec<PromptTemplate>, sqlx::Error> {
    sqlx::query_as::<_, PromptTemplateRow>(
        r"
        SELECT id, name, category, content, variables
        FROM prompt_templates
        ORDER BY category, name
        "
    )
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(|r| r.into()).collect())
}

/// Get a prompt template by ID
pub async fn get_template(pool: &PgPool, id: &str) -> Result<Option<PromptTemplate>, sqlx::Error> {
    sqlx::query_as::<_, PromptTemplateRow>(
        r"
        SELECT id, name, category, content, variables
        FROM prompt_templates
        WHERE id = $1
        "
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map(|opt| opt.map(|r| r.into()))
}

/// Create a prompt template
pub async fn create_template(pool: &PgPool, template: &PromptTemplate) -> Result<PromptTemplate, sqlx::Error> {
    sqlx::query_as::<_, PromptTemplateRow>(
        r"
        INSERT INTO prompt_templates (id, name, category, content, variables)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, name, category, content, variables
        "
    )
    .bind(&template.id)
    .bind(&template.name)
    .bind(&template.category)
    .bind(&template.content)
    .bind(&template.variables)
    .fetch_one(pool)
    .await
    .map(|r| r.into())
}

/// Update a prompt template
pub async fn update_template(pool: &PgPool, id: &str, template: &PromptTemplate) -> Result<PromptTemplate, sqlx::Error> {
    sqlx::query_as::<_, PromptTemplateRow>(
        r"
        UPDATE prompt_templates
        SET name = $2, category = $3, content = $4, variables = $5, updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, category, content, variables
        "
    )
    .bind(id)
    .bind(&template.name)
    .bind(&template.category)
    .bind(&template.content)
    .bind(&template.variables)
    .fetch_one(pool)
    .await
    .map(|r| r.into())
}

/// Delete a prompt template
pub async fn delete_template(pool: &PgPool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM prompt_templates WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Internal row type for PromptTemplate
#[derive(sqlx::FromRow)]
struct PromptTemplateRow {
    id: String,
    name: String,
    category: String,
    content: String,
    variables: Vec<String>,
}

impl From<PromptTemplateRow> for PromptTemplate {
    fn from(row: PromptTemplateRow) -> Self {
        PromptTemplate {
            id: row.id,
            name: row.name,
            category: row.category,
            content: row.content,
            variables: row.variables,
        }
    }
}

// ============== Global AI Config ==============

use crate::models::GlobalAiConfig;

/// Get global AI configuration
pub async fn get_global_config(pool: &PgPool) -> Result<GlobalAiConfig, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM global_ai_config"
    )
    .fetch_all(pool)
    .await?;

    let mut config = GlobalAiConfig::default();
    for (key, value) in rows {
        match key.as_str() {
            "model" => config.model = value,
            "use_claude_code" => config.use_claude_code = value.parse().unwrap_or(true),
            "fallback_to_api" => config.fallback_to_api = value.parse().unwrap_or(true),
            "default_voice" => config.default_voice = value,
            "max_call_duration" => config.max_call_duration = value.parse().unwrap_or(300),
            "stt_provider" => config.stt_provider = value,
            "tts_provider" => config.tts_provider = value,
            _ => {}
        }
    }
    Ok(config)
}

/// Update global AI configuration
pub async fn update_global_config(pool: &PgPool, config: &GlobalAiConfig) -> Result<(), sqlx::Error> {
    let updates = vec![
        ("model", config.model.clone()),
        ("use_claude_code", config.use_claude_code.to_string()),
        ("fallback_to_api", config.fallback_to_api.to_string()),
        ("default_voice", config.default_voice.clone()),
        ("max_call_duration", config.max_call_duration.to_string()),
        ("stt_provider", config.stt_provider.clone()),
        ("tts_provider", config.tts_provider.clone()),
    ];

    for (key, value) in updates {
        sqlx::query(
            r"
            INSERT INTO global_ai_config (key, value, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()
            "
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    }

    Ok(())
}

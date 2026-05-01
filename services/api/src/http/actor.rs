use axum::{extract::FromRequestParts, http::request::Parts};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::{http::error::ApiError, state::AppState};

pub const SESSION_COOKIE_NAME: &str = "aw_session";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    Human,
    Agent,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorContext {
    pub actor_kind: ActorKind,
    pub actor_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl ActorContext {
    pub fn system() -> Self {
        Self {
            actor_kind: ActorKind::System,
            actor_id: "anonymous".to_string(),
            workspace_id: None,
            project_id: None,
            role: None,
            scopes: Vec::new(),
        }
    }
}

pub fn hash_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex::encode(digest)
}

pub fn utc_now_text() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

impl FromRequestParts<AppState> for ActorContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let request_id = request_id(parts);

        if let Some(token) = session_cookie(parts) {
            return actor_from_session(&state.pool, &token, &request_id).await;
        }

        if let Some(token) = bearer_token(parts) {
            return actor_from_bearer(&state.pool, &token, &request_id).await;
        }

        Ok(actor_from_legacy_headers(parts))
    }
}

fn request_id(parts: &Parts) -> String {
    parts
        .headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("missing")
        .to_string()
}

fn session_cookie(parts: &Parts) -> Option<String> {
    let raw = parts.headers.get("cookie")?.to_str().ok()?;
    raw.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == SESSION_COOKIE_NAME && !value.is_empty()).then(|| value.to_string())
    })
}

fn bearer_token(parts: &Parts) -> Option<String> {
    let raw = parts.headers.get("authorization")?.to_str().ok()?;
    raw.strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
}

fn actor_from_legacy_headers(parts: &Parts) -> ActorContext {
    let actor_kind = parts
        .headers
        .get("x-actor-kind")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| match s {
            "human" => Some(ActorKind::Human),
            "agent" => Some(ActorKind::Agent),
            "system" => Some(ActorKind::System),
            _ => None,
        })
        .unwrap_or(ActorKind::System);

    let actor_id = parts
        .headers
        .get("x-actor-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();

    let workspace_id = parts
        .headers
        .get("x-workspace-id")
        .and_then(|v| v.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let project_id = parts
        .headers
        .get("x-project-id")
        .and_then(|v| v.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let role = parts
        .headers
        .get("x-actor-role")
        .and_then(|v| v.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let scopes = parts
        .headers
        .get("x-actor-scopes")
        .and_then(|v| v.to_str().ok())
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();

    ActorContext {
        actor_kind,
        actor_id,
        workspace_id,
        project_id,
        role,
        scopes,
    }
}

async fn actor_from_session(
    pool: &sqlx::AnyPool,
    token: &str,
    request_id: &str,
) -> Result<ActorContext, ApiError> {
    let token_hash = hash_secret(token);
    let now = utc_now_text();

    let row = sqlx::query_as::<_, HumanSessionActorRow>(
        "SELECT CAST(hs.workspace_member_id AS TEXT) AS member_id,
                CAST(wm.workspace_id AS TEXT) AS workspace_id,
                wm.role AS role
         FROM human_sessions hs
         JOIN workspace_members wm ON wm.id = hs.workspace_member_id
         WHERE hs.token_hash = $1
           AND hs.revoked_at IS NULL
           AND CAST(hs.expires_at AS TEXT) > $2
           AND wm.status = 'active'",
    )
    .bind(token_hash)
    .bind(now)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "session actor lookup failed");
        ApiError::internal(request_id, "failed to resolve session")
    })?;

    let row = row.ok_or_else(|| ApiError::unauthorised(request_id, "session is not valid"))?;

    if let Err(e) = sqlx::query(
        "UPDATE human_sessions
         SET last_seen_at = CURRENT_TIMESTAMP
         WHERE token_hash = $1",
    )
    .bind(hash_secret(token))
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "failed to update session last_seen_at");
    }

    Ok(ActorContext {
        actor_kind: ActorKind::Human,
        actor_id: row.member_id,
        workspace_id: Some(row.workspace_id),
        project_id: None,
        role: Some(row.role),
        scopes: Vec::new(),
    })
}

async fn actor_from_bearer(
    pool: &sqlx::AnyPool,
    token: &str,
    request_id: &str,
) -> Result<ActorContext, ApiError> {
    let token_hash = hash_secret(token);
    let now = utc_now_text();

    let row = sqlx::query_as::<_, AgentCredentialActorRow>(
        "SELECT CAST(ac.agent_id AS TEXT) AS agent_id,
                CAST(ac.workspace_id AS TEXT) AS workspace_id,
                CAST(ac.project_id AS TEXT) AS project_id,
                CAST(ac.scope_policy AS TEXT) AS scope_policy
         FROM agent_credentials ac
         JOIN agents a ON a.id = ac.agent_id
         WHERE ac.secret_hash = $1
           AND ac.status = 'active'
           AND a.status = 'active'
           AND (ac.expires_at IS NULL OR CAST(ac.expires_at AS TEXT) > $2)",
    )
    .bind(token_hash)
    .bind(now)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "agent bearer lookup failed");
        ApiError::internal(request_id, "failed to resolve bearer credential")
    })?;

    let row = row.ok_or_else(|| ApiError::unauthorised(request_id, "bearer token is not valid"))?;

    Ok(ActorContext {
        actor_kind: ActorKind::Agent,
        actor_id: row.agent_id,
        workspace_id: Some(row.workspace_id),
        project_id: row.project_id,
        role: None,
        scopes: parse_scopes(&row.scope_policy),
    })
}

fn parse_scopes(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

#[derive(sqlx::FromRow)]
struct HumanSessionActorRow {
    member_id: String,
    workspace_id: String,
    role: String,
}

#[derive(sqlx::FromRow)]
struct AgentCredentialActorRow {
    agent_id: String,
    workspace_id: String,
    project_id: Option<String>,
    scope_policy: String,
}

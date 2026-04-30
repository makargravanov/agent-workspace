use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    http::{
        actor::{hash_secret, utc_now_text, ActorContext, SESSION_COOKIE_NAME},
        error::ApiError,
        request_id::RequestId,
        response::{ApiResponse, ResponseMeta},
    },
    state::AppState,
};

const DEFAULT_DEV_WORKSPACE_SLUG: &str = "dev-workspace";
const DEFAULT_DEV_WORKSPACE_NAME: &str = "Dev Workspace";
const DEFAULT_DEV_SUBJECT: &str = "dev:owner-1";
const DEFAULT_DEV_DISPLAY_NAME: &str = "Dev Owner";
const SESSION_TTL_DAYS: i64 = 7;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/session", get(session))
        .route("/auth/dev/login", post(dev_login))
        .route("/auth/logout", post(logout))
}

#[derive(Debug, Deserialize)]
pub struct DevLoginRequest {
    pub workspace_slug: Option<String>,
    pub workspace_name: Option<String>,
    pub external_subject: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<ActorContext>,
}

#[derive(sqlx::FromRow)]
struct WorkspaceRow {
    id: String,
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    id: String,
}

async fn dev_login(
    State(state): State<AppState>,
    request_id: RequestId,
    headers: HeaderMap,
    Json(body): Json<DevLoginRequest>,
) -> Result<(HeaderMap, ApiResponse<SessionResponse>), ApiError> {
    let workspace_slug = body
        .workspace_slug
        .unwrap_or_else(|| DEFAULT_DEV_WORKSPACE_SLUG.to_string());
    let workspace_name = body
        .workspace_name
        .unwrap_or_else(|| DEFAULT_DEV_WORKSPACE_NAME.to_string());
    let external_subject = body
        .external_subject
        .unwrap_or_else(|| DEFAULT_DEV_SUBJECT.to_string());
    let display_name = body
        .display_name
        .unwrap_or_else(|| DEFAULT_DEV_DISPLAY_NAME.to_string());

    validate_dev_input(
        &workspace_slug,
        &external_subject,
        &display_name,
        &request_id,
    )?;

    let workspace_id =
        ensure_workspace(&state.pool, &workspace_slug, &workspace_name, &request_id).await?;
    let member_id = ensure_member(
        &state.pool,
        &workspace_id,
        &external_subject,
        &display_name,
        &request_id,
    )
    .await?;
    ensure_identity(
        &state.pool,
        &member_id,
        &external_subject,
        &display_name,
        &request_id,
    )
    .await?;

    let token = format!("awsess_{}_{}", Uuid::new_v4(), Uuid::new_v4());
    let token_hash = hash_secret(&token);
    let expires_at = utc_text(OffsetDateTime::now_utc() + Duration::days(SESSION_TTL_DAYS));
    let session_id = Uuid::new_v4().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

    let session_insert_sql = if is_postgres(&state.pool).await {
        "INSERT INTO human_sessions
         (id, workspace_member_id, token_hash, expires_at, user_agent)
         VALUES (CAST($1 AS UUID), CAST($2 AS UUID), $3, CAST($4 AS TIMESTAMPTZ), $5)"
    } else {
        "INSERT INTO human_sessions
         (id, workspace_member_id, token_hash, expires_at, user_agent)
         VALUES ($1, $2, $3, $4, $5)"
    };

    sqlx::query(session_insert_sql)
        .bind(session_id)
        .bind(&member_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(user_agent)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "dev_login session insert failed");
            ApiError::internal(&request_id.0, "failed to create session")
        })?;

    let actor = ActorContext {
        actor_kind: crate::http::actor::ActorKind::Human,
        actor_id: member_id,
        workspace_id: Some(workspace_id),
        project_id: None,
        role: Some("owner".to_string()),
        scopes: Vec::new(),
    };

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie(&token))
            .map_err(|_| ApiError::internal(&request_id.0, "failed to build session cookie"))?,
    );

    Ok((
        response_headers,
        ApiResponse {
            data: SessionResponse {
                authenticated: true,
                actor: Some(actor),
            },
            meta: ResponseMeta {
                request_id: request_id.0,
                audit_event_id: None,
            },
        },
    ))
}

async fn session(
    request_id: RequestId,
    actor: ActorContext,
) -> Result<ApiResponse<SessionResponse>, ApiError> {
    let authenticated = actor.actor_kind != crate::http::actor::ActorKind::System;
    Ok(ApiResponse {
        data: SessionResponse {
            authenticated,
            actor: authenticated.then_some(actor),
        },
        meta: ResponseMeta {
            request_id: request_id.0,
            audit_event_id: None,
        },
    })
}

async fn logout(
    State(state): State<AppState>,
    request_id: RequestId,
    headers: HeaderMap,
) -> Result<(StatusCode, HeaderMap, ApiResponse<SessionResponse>), ApiError> {
    if let Some(token) = session_cookie_from_headers(&headers) {
        let token_hash = hash_secret(&token);
        let revoke_sql = if is_postgres(&state.pool).await {
            "UPDATE human_sessions
             SET revoked_at = CAST($1 AS TIMESTAMPTZ)
             WHERE token_hash = $2 AND revoked_at IS NULL"
        } else {
            "UPDATE human_sessions
             SET revoked_at = $1
             WHERE token_hash = $2 AND revoked_at IS NULL"
        };

        sqlx::query(revoke_sql)
        .bind(utc_now_text())
        .bind(token_hash)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "logout session revoke failed");
            ApiError::internal(&request_id.0, "failed to revoke session")
        })?;
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_static("aw_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax"),
    );

    Ok((
        StatusCode::OK,
        response_headers,
        ApiResponse {
            data: SessionResponse {
                authenticated: false,
                actor: None,
            },
            meta: ResponseMeta {
                request_id: request_id.0,
                audit_event_id: None,
            },
        },
    ))
}

async fn ensure_workspace(
    pool: &sqlx::AnyPool,
    slug: &str,
    name: &str,
    request_id: &RequestId,
) -> Result<String, ApiError> {
    if let Some(row) = sqlx::query_as::<_, WorkspaceRow>(
        "SELECT CAST(id AS TEXT) AS id FROM workspaces WHERE slug = $1",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "dev_login workspace lookup failed");
        ApiError::internal(&request_id.0, "failed to resolve workspace")
    })? {
        return Ok(row.id);
    }

    let id = Uuid::new_v4().to_string();
    let insert_sql = if is_postgres(pool).await {
        "INSERT INTO workspaces (id, slug, name) VALUES (CAST($1 AS UUID), $2, $3)"
    } else {
        "INSERT INTO workspaces (id, slug, name) VALUES ($1, $2, $3)"
    };

    sqlx::query(insert_sql)
        .bind(&id)
        .bind(slug)
        .bind(name)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "dev_login workspace insert failed");
            ApiError::internal(&request_id.0, "failed to create workspace")
        })?;

    Ok(id)
}

async fn ensure_member(
    pool: &sqlx::AnyPool,
    workspace_id: &str,
    external_subject: &str,
    display_name: &str,
    request_id: &RequestId,
) -> Result<String, ApiError> {
    let member_lookup_sql = if is_postgres(pool).await {
        "SELECT CAST(id AS TEXT) AS id
         FROM workspace_members
         WHERE workspace_id = CAST($1 AS UUID) AND external_subject = $2"
    } else {
        "SELECT CAST(id AS TEXT) AS id
         FROM workspace_members
         WHERE workspace_id = $1 AND external_subject = $2"
    };

    if let Some(row) = sqlx::query_as::<_, MemberRow>(member_lookup_sql)
        .bind(workspace_id)
        .bind(external_subject)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "dev_login member lookup failed");
            ApiError::internal(&request_id.0, "failed to resolve workspace member")
        })?
    {
        return Ok(row.id);
    }

    let id = Uuid::new_v4().to_string();
    let insert_sql = if is_postgres(pool).await {
        "INSERT INTO workspace_members
         (id, workspace_id, external_subject, display_name, role, status)
         VALUES (CAST($1 AS UUID), CAST($2 AS UUID), $3, $4, 'owner', 'active')"
    } else {
        "INSERT INTO workspace_members
         (id, workspace_id, external_subject, display_name, role, status)
         VALUES ($1, $2, $3, $4, 'owner', 'active')"
    };

    sqlx::query(insert_sql)
        .bind(&id)
        .bind(workspace_id)
        .bind(external_subject)
        .bind(display_name)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "dev_login member insert failed");
            ApiError::internal(&request_id.0, "failed to create workspace member")
        })?;

    Ok(id)
}

async fn ensure_identity(
    pool: &sqlx::AnyPool,
    member_id: &str,
    external_subject: &str,
    display_name: &str,
    request_id: &RequestId,
) -> Result<(), ApiError> {
    let exists: Option<(String,)> = sqlx::query_as(
        "SELECT CAST(id AS TEXT) FROM human_identities
         WHERE provider = 'dev' AND provider_subject = $1",
    )
    .bind(external_subject)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "dev_login identity lookup failed");
        ApiError::internal(&request_id.0, "failed to resolve human identity")
    })?;

    if exists.is_some() {
        return Ok(());
    }

    let insert_sql = if is_postgres(pool).await {
        "INSERT INTO human_identities
         (id, workspace_member_id, provider, provider_subject, display_name)
         VALUES (CAST($1 AS UUID), CAST($2 AS UUID), 'dev', $3, $4)"
    } else {
        "INSERT INTO human_identities
         (id, workspace_member_id, provider, provider_subject, display_name)
         VALUES ($1, $2, 'dev', $3, $4)"
    };

    sqlx::query(insert_sql)
        .bind(Uuid::new_v4().to_string())
        .bind(member_id)
        .bind(external_subject)
        .bind(display_name)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "dev_login identity insert failed");
            ApiError::internal(&request_id.0, "failed to create human identity")
        })?;

    Ok(())
}

fn validate_dev_input(
    workspace_slug: &str,
    external_subject: &str,
    display_name: &str,
    request_id: &RequestId,
) -> Result<(), ApiError> {
    if workspace_slug.is_empty()
        || !workspace_slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ApiError::validation_error(
            &request_id.0,
            "workspace_slug must be lowercase kebab-case",
        ));
    }

    if external_subject.trim().is_empty() || display_name.trim().is_empty() {
        return Err(ApiError::validation_error(
            &request_id.0,
            "external_subject and display_name must not be empty",
        ));
    }

    Ok(())
}

fn session_cookie(token: &str) -> String {
    let secure = std::env::var("SESSION_COOKIE_SECURE")
        .map(|value| value != "false")
        .unwrap_or(true);
    let secure_attr = if secure { "; Secure" } else { "" };
    format!(
        "{SESSION_COOKIE_NAME}={token}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax{}",
        SESSION_TTL_DAYS * 24 * 60 * 60,
        secure_attr
    )
}

fn session_cookie_from_headers(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == SESSION_COOKIE_NAME && !value.is_empty()).then(|| value.to_string())
    })
}

async fn is_postgres(pool: &sqlx::AnyPool) -> bool {
    sqlx::query_scalar::<_, String>("SELECT sqlite_version()")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .is_none()
}

fn utc_text(value: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        value.year(),
        u8::from(value.month()),
        value.day(),
        value.hour(),
        value.minute(),
        value.second()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::build_router, testing::any_test_pool};
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tower::ServiceExt;

    async fn app() -> axum::Router {
        build_router(AppState::new(any_test_pool().await))
    }

    async fn body_json(body: Body) -> Value {
        let bytes = to_bytes(body, usize::MAX).await.expect("body bytes");
        serde_json::from_slice(&bytes).expect("json body")
    }

    #[tokio::test]
    async fn dev_login_sets_cookie_and_session_reads_actor() {
        let app = app().await;

        let login = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/dev/login")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({}).to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(login.status(), StatusCode::OK);
        let cookie = login
            .headers()
            .get(header::SET_COOKIE)
            .expect("set-cookie")
            .to_str()
            .expect("cookie header")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string();

        let session = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/session")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(session.status(), StatusCode::OK);
        let body = body_json(session.into_body()).await;
        assert_eq!(body["data"]["authenticated"], true);
        assert_eq!(body["data"]["actor"]["actor_kind"], "human");
        assert_eq!(body["data"]["actor"]["role"], "owner");
    }

    #[tokio::test]
    async fn logout_revokes_session() {
        let app = app().await;

        let login = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/dev/login")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({}).to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");

        let cookie = login
            .headers()
            .get(header::SET_COOKIE)
            .expect("set-cookie")
            .to_str()
            .expect("cookie header")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string();

        let logout = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/logout")
                    .header(header::COOKIE, cookie.clone())
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(logout.status(), StatusCode::OK);

        let session = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/session")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(session.status(), StatusCode::UNAUTHORIZED);
    }
}

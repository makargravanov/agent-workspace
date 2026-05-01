use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Redirect,
    routing::{get, post},
    Json, Router,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    db::DatabaseBackend,
    http::{
        actor::{hash_secret, utc_now_text, ActorContext, ActorKind, SESSION_COOKIE_NAME},
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
const GITHUB_OAUTH_STATE_COOKIE_NAME: &str = "aw_github_state";
const GITHUB_OAUTH_STATE_TTL_SECONDS: i64 = 10 * 60;
const DEFAULT_GITHUB_AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const DEFAULT_GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const DEFAULT_GITHUB_USER_URL: &str = "https://api.github.com/user";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/session", get(session))
        .route("/auth/github/start", get(github_start))
        .route("/auth/github/callback", get(github_callback))
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

#[derive(Debug, Deserialize)]
struct GithubCallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<ActorContext>,
}

#[derive(Debug, Clone)]
struct GithubOAuthConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    success_redirect_path: String,
    authorize_url: String,
    token_url: String,
    user_url: String,
}

#[derive(Debug, Clone)]
struct GithubProfile {
    provider_subject: String,
    login: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct GithubTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GithubUserResponse {
    id: u64,
    login: String,
    name: Option<String>,
}

#[derive(sqlx::FromRow)]
struct WorkspaceRow {
    id: String,
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    id: String,
}

#[derive(sqlx::FromRow)]
struct SessionActorRow {
    workspace_id: String,
    role: String,
}

async fn github_start(request_id: RequestId) -> Result<(HeaderMap, Redirect), ApiError> {
    let cfg = GithubOAuthConfig::from_env(&request_id.0)?;
    let state = format!("gho_{}_{}", Uuid::new_v4(), Uuid::new_v4());
    let redirect_url = Url::parse_with_params(
        &cfg.authorize_url,
        &[
            ("client_id", cfg.client_id.as_str()),
            ("redirect_uri", cfg.redirect_uri.as_str()),
            ("state", state.as_str()),
            ("scope", "read:user"),
        ],
    )
    .map_err(|e| ApiError::internal(&request_id.0, format!("failed to build GitHub OAuth URL: {e}")))?;

    let mut response_headers = HeaderMap::new();
    response_headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&state_cookie(&state))
            .map_err(|_| ApiError::internal(&request_id.0, "failed to build oauth state cookie"))?,
    );

    Ok((response_headers, Redirect::to(redirect_url.as_str())))
}

async fn github_callback(
    State(state): State<AppState>,
    request_id: RequestId,
    headers: HeaderMap,
    Query(query): Query<GithubCallbackQuery>,
) -> Result<(HeaderMap, Redirect), ApiError> {
    let code = query.code.ok_or_else(|| {
        ApiError::validation_error(&request_id.0, "missing GitHub OAuth code")
    })?;
    let returned_state = query.state.ok_or_else(|| {
        ApiError::validation_error(&request_id.0, "missing GitHub OAuth state")
    })?;
    let expected_state = oauth_state_cookie_from_headers(&headers).ok_or_else(|| {
        ApiError::unauthorised(&request_id.0, "missing GitHub OAuth state cookie")
    })?;

    if returned_state != expected_state {
        return Err(ApiError::unauthorised(
            &request_id.0,
            "GitHub OAuth state did not match",
        ));
    }

    let cfg = GithubOAuthConfig::from_env(&request_id.0)?;
    let profile = exchange_github_code(&cfg, &code, &request_id.0).await?;
    let member_id = resolve_or_create_github_member(
        &state.pool,
        state.db_backend,
        &profile,
        &request_id,
    )
    .await?;
    let session_cookie_value = create_human_session(
        &state.pool,
        state.db_backend,
        &headers,
        &member_id,
        &request_id,
    )
    .await?;

    let mut response_headers = HeaderMap::new();
    response_headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie(&session_cookie_value))
            .map_err(|_| ApiError::internal(&request_id.0, "failed to build session cookie"))?,
    );
    response_headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&expired_cookie(GITHUB_OAUTH_STATE_COOKIE_NAME))
            .map_err(|_| ApiError::internal(&request_id.0, "failed to clear oauth state cookie"))?,
    );

    Ok((response_headers, Redirect::to(&cfg.success_redirect_path)))
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

    let workspace_id = ensure_workspace(
        &state.pool,
        state.db_backend,
        &workspace_slug,
        &workspace_name,
        &request_id,
    )
    .await?;
    let member_id = ensure_member(
        &state.pool,
        state.db_backend,
        &workspace_id,
        &external_subject,
        &display_name,
        &request_id,
    )
    .await?;
    ensure_identity(
        &state.pool,
        state.db_backend,
        &member_id,
        "dev",
        &external_subject,
        &display_name,
        &request_id,
    )
    .await?;

    let session_cookie_value = create_human_session(
        &state.pool,
        state.db_backend,
        &headers,
        &member_id,
        &request_id,
    )
    .await?;
    let actor = actor_for_member(&state.pool, state.db_backend, &member_id, &request_id).await?;

    let mut response_headers = HeaderMap::new();
    response_headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie(&session_cookie_value))
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
    let authenticated = actor.actor_kind != ActorKind::System;
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
        let revoke_sql = if state.db_backend == DatabaseBackend::Postgres {
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
    response_headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&expired_cookie(SESSION_COOKIE_NAME))
            .map_err(|_| ApiError::internal(&request_id.0, "failed to clear session cookie"))?,
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

async fn exchange_github_code(
    cfg: &GithubOAuthConfig,
    code: &str,
    request_id: &str,
) -> Result<GithubProfile, ApiError> {
    let client = reqwest::Client::builder()
        .user_agent("agent-workspace-api")
        .build()
        .map_err(|e| ApiError::internal(request_id, format!("failed to build GitHub client: {e}")))?;

    let token_response = client
        .post(&cfg.token_url)
        .header(header::ACCEPT.as_str(), "application/json")
        .form(&[
            ("client_id", cfg.client_id.as_str()),
            ("client_secret", cfg.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", cfg.redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|e| ApiError::internal(request_id, format!("GitHub token exchange failed: {e}")))?;

    if !token_response.status().is_success() {
        return Err(ApiError::internal(
            request_id,
            format!("GitHub token exchange returned {}", token_response.status()),
        ));
    }

    let token_body = token_response
        .json::<GithubTokenResponse>()
        .await
        .map_err(|e| ApiError::internal(request_id, format!("failed to parse GitHub token response: {e}")))?;

    let user_response = client
        .get(&cfg.user_url)
        .bearer_auth(&token_body.access_token)
        .header(header::ACCEPT.as_str(), "application/json")
        .send()
        .await
        .map_err(|e| ApiError::internal(request_id, format!("GitHub user lookup failed: {e}")))?;

    if !user_response.status().is_success() {
        return Err(ApiError::internal(
            request_id,
            format!("GitHub user lookup returned {}", user_response.status()),
        ));
    }

    let user = user_response
        .json::<GithubUserResponse>()
        .await
        .map_err(|e| ApiError::internal(request_id, format!("failed to parse GitHub user response: {e}")))?;

    Ok(GithubProfile {
        provider_subject: format!("github:user:{}", user.id),
        login: user.login.clone(),
        display_name: user.name.unwrap_or(user.login),
    })
}

async fn resolve_or_create_github_member(
    pool: &sqlx::AnyPool,
    db_backend: DatabaseBackend,
    profile: &GithubProfile,
    request_id: &RequestId,
) -> Result<String, ApiError> {
    if let Some(member_id) = resolve_member_by_identity(
        pool,
        db_backend,
        "github",
        &profile.provider_subject,
        request_id,
    )
    .await?
    {
        sync_member_display_name(pool, db_backend, &member_id, &profile.display_name, request_id)
            .await?;
        ensure_identity(
            pool,
            db_backend,
            &member_id,
            "github",
            &profile.provider_subject,
            &profile.display_name,
            request_id,
        )
        .await?;
        return Ok(member_id);
    }

    if let Some(member_id) = resolve_member_by_external_subject(
        pool,
        db_backend,
        &profile.provider_subject,
        request_id,
    )
    .await?
    {
        sync_member_display_name(pool, db_backend, &member_id, &profile.display_name, request_id)
            .await?;
        ensure_identity(
            pool,
            db_backend,
            &member_id,
            "github",
            &profile.provider_subject,
            &profile.display_name,
            request_id,
        )
        .await?;
        return Ok(member_id);
    }

    let workspace_slug = github_workspace_slug(&profile.login, &profile.provider_subject);
    let workspace_name = format!("{} Workspace", profile.display_name);
    let workspace_id =
        ensure_workspace(pool, db_backend, &workspace_slug, &workspace_name, request_id).await?;
    let member_id = ensure_member(
        pool,
        db_backend,
        &workspace_id,
        &profile.provider_subject,
        &profile.display_name,
        request_id,
    )
    .await?;
    ensure_identity(
        pool,
        db_backend,
        &member_id,
        "github",
        &profile.provider_subject,
        &profile.display_name,
        request_id,
    )
    .await?;

    Ok(member_id)
}

async fn create_human_session(
    pool: &sqlx::AnyPool,
    db_backend: DatabaseBackend,
    headers: &HeaderMap,
    member_id: &str,
    request_id: &RequestId,
) -> Result<String, ApiError> {
    let token = format!("awsess_{}_{}", Uuid::new_v4(), Uuid::new_v4());
    let token_hash = hash_secret(&token);
    let expires_at = utc_text(OffsetDateTime::now_utc() + Duration::days(SESSION_TTL_DAYS));
    let session_id = Uuid::new_v4().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);
    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let session_insert_sql = if db_backend == DatabaseBackend::Postgres {
        "INSERT INTO human_sessions
         (id, workspace_member_id, token_hash, expires_at, user_agent, ip_address)
         VALUES (CAST($1 AS UUID), CAST($2 AS UUID), $3, CAST($4 AS TIMESTAMPTZ), $5, $6)"
    } else {
        "INSERT INTO human_sessions
         (id, workspace_member_id, token_hash, expires_at, user_agent, ip_address)
         VALUES ($1, $2, $3, $4, $5, $6)"
    };

    sqlx::query(session_insert_sql)
        .bind(session_id)
        .bind(member_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(user_agent)
        .bind(ip_address)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, member_id = %member_id, "session insert failed");
            ApiError::internal(&request_id.0, "failed to create session")
        })?;

    Ok(token)
}

async fn actor_for_member(
    pool: &sqlx::AnyPool,
    db_backend: DatabaseBackend,
    member_id: &str,
    request_id: &RequestId,
) -> Result<ActorContext, ApiError> {
    let query = if db_backend == DatabaseBackend::Postgres {
        "SELECT CAST(workspace_id AS TEXT) AS workspace_id, role
         FROM workspace_members
         WHERE id = CAST($1 AS UUID) AND status = 'active'"
    } else {
        "SELECT CAST(workspace_id AS TEXT) AS workspace_id, role
         FROM workspace_members
         WHERE id = $1 AND status = 'active'"
    };

    let row = sqlx::query_as::<_, SessionActorRow>(query)
        .bind(member_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, member_id = %member_id, "member actor lookup failed");
            ApiError::internal(&request_id.0, "failed to resolve session actor")
        })?
        .ok_or_else(|| ApiError::unauthorised(&request_id.0, "workspace member is not active"))?;

    Ok(ActorContext {
        actor_kind: ActorKind::Human,
        actor_id: member_id.to_string(),
        workspace_id: Some(row.workspace_id),
        project_id: None,
        role: Some(row.role),
        scopes: Vec::new(),
    })
}

async fn ensure_workspace(
    pool: &sqlx::AnyPool,
    db_backend: DatabaseBackend,
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
        tracing::error!(error = %e, "workspace lookup failed");
        ApiError::internal(&request_id.0, "failed to resolve workspace")
    })? {
        return Ok(row.id);
    }

    let id = Uuid::new_v4().to_string();
    let insert_sql = if db_backend == DatabaseBackend::Postgres {
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
            tracing::error!(error = %e, "workspace insert failed");
            ApiError::internal(&request_id.0, "failed to create workspace")
        })?;

    Ok(id)
}

async fn ensure_member(
    pool: &sqlx::AnyPool,
    db_backend: DatabaseBackend,
    workspace_id: &str,
    external_subject: &str,
    display_name: &str,
    request_id: &RequestId,
) -> Result<String, ApiError> {
    let member_lookup_sql = if db_backend == DatabaseBackend::Postgres {
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
            tracing::error!(error = %e, "member lookup failed");
            ApiError::internal(&request_id.0, "failed to resolve workspace member")
        })?
    {
        return Ok(row.id);
    }

    let id = Uuid::new_v4().to_string();
    let insert_sql = if db_backend == DatabaseBackend::Postgres {
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
            tracing::error!(error = %e, "member insert failed");
            ApiError::internal(&request_id.0, "failed to create workspace member")
        })?;

    Ok(id)
}

async fn resolve_member_by_identity(
    pool: &sqlx::AnyPool,
    db_backend: DatabaseBackend,
    provider: &str,
    provider_subject: &str,
    request_id: &RequestId,
) -> Result<Option<String>, ApiError> {
    let query = if db_backend == DatabaseBackend::Postgres {
        "SELECT CAST(hi.workspace_member_id AS TEXT) AS id
         FROM human_identities hi
         JOIN workspace_members wm ON wm.id = hi.workspace_member_id
         WHERE hi.provider = $1
           AND hi.provider_subject = $2
           AND wm.status = 'active'
         LIMIT 1"
    } else {
        "SELECT CAST(hi.workspace_member_id AS TEXT) AS id
         FROM human_identities hi
         JOIN workspace_members wm ON wm.id = hi.workspace_member_id
         WHERE hi.provider = $1
           AND hi.provider_subject = $2
           AND wm.status = 'active'
         LIMIT 1"
    };

    let row = sqlx::query_as::<_, MemberRow>(query)
        .bind(provider)
        .bind(provider_subject)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, provider = %provider, provider_subject = %provider_subject, "identity lookup failed");
            ApiError::internal(&request_id.0, "failed to resolve human identity")
        })?;

    Ok(row.map(|row| row.id))
}

async fn resolve_member_by_external_subject(
    pool: &sqlx::AnyPool,
    _db_backend: DatabaseBackend,
    external_subject: &str,
    request_id: &RequestId,
) -> Result<Option<String>, ApiError> {
    let row = sqlx::query_as::<_, MemberRow>(
        "SELECT CAST(id AS TEXT) AS id
         FROM workspace_members
         WHERE external_subject = $1
           AND status = 'active'
         ORDER BY created_at
         LIMIT 1",
    )
    .bind(external_subject)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, external_subject = %external_subject, "member lookup by subject failed");
        ApiError::internal(&request_id.0, "failed to resolve workspace member")
    })?;

    Ok(row.map(|row| row.id))
}

async fn ensure_identity(
    pool: &sqlx::AnyPool,
    db_backend: DatabaseBackend,
    member_id: &str,
    provider: &str,
    provider_subject: &str,
    display_name: &str,
    request_id: &RequestId,
) -> Result<(), ApiError> {
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT CAST(workspace_member_id AS TEXT) FROM human_identities
         WHERE provider = $1 AND provider_subject = $2",
    )
    .bind(provider)
    .bind(provider_subject)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, provider = %provider, provider_subject = %provider_subject, "identity lookup failed");
        ApiError::internal(&request_id.0, "failed to resolve human identity")
    })?;

    match existing {
        Some((existing_member_id,)) if existing_member_id == member_id => {
            let update_sql = if db_backend == DatabaseBackend::Postgres {
                "UPDATE human_identities
                 SET display_name = $1, updated_at = CAST($2 AS TIMESTAMPTZ)
                 WHERE provider = $3 AND provider_subject = $4"
            } else {
                "UPDATE human_identities
                 SET display_name = $1, updated_at = $2
                 WHERE provider = $3 AND provider_subject = $4"
            };

            sqlx::query(update_sql)
                .bind(display_name)
                .bind(utc_now_text())
                .bind(provider)
                .bind(provider_subject)
                .execute(pool)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, provider = %provider, provider_subject = %provider_subject, "identity update failed");
                    ApiError::internal(&request_id.0, "failed to update human identity")
                })?;

            Ok(())
        }
        Some((_other_member_id,)) => Err(ApiError::forbidden(
            &request_id.0,
            "human identity is already linked to another workspace member",
        )),
        None => {
            let insert_sql = if db_backend == DatabaseBackend::Postgres {
                "INSERT INTO human_identities
                 (id, workspace_member_id, provider, provider_subject, display_name)
                 VALUES (CAST($1 AS UUID), CAST($2 AS UUID), $3, $4, $5)"
            } else {
                "INSERT INTO human_identities
                 (id, workspace_member_id, provider, provider_subject, display_name)
                 VALUES ($1, $2, $3, $4, $5)"
            };

            sqlx::query(insert_sql)
                .bind(Uuid::new_v4().to_string())
                .bind(member_id)
                .bind(provider)
                .bind(provider_subject)
                .bind(display_name)
                .execute(pool)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, provider = %provider, provider_subject = %provider_subject, "identity insert failed");
                    ApiError::internal(&request_id.0, "failed to create human identity")
                })?;

            Ok(())
        }
    }
}

async fn sync_member_display_name(
    pool: &sqlx::AnyPool,
    db_backend: DatabaseBackend,
    member_id: &str,
    display_name: &str,
    request_id: &RequestId,
) -> Result<(), ApiError> {
    let update_sql = if db_backend == DatabaseBackend::Postgres {
        "UPDATE workspace_members
         SET display_name = $1, updated_at = CAST($2 AS TIMESTAMPTZ)
         WHERE id = CAST($3 AS UUID)"
    } else {
        "UPDATE workspace_members
         SET display_name = $1, updated_at = $2
         WHERE id = $3"
    };

    sqlx::query(update_sql)
        .bind(display_name)
        .bind(utc_now_text())
        .bind(member_id)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, member_id = %member_id, "member display_name update failed");
            ApiError::internal(&request_id.0, "failed to update workspace member")
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

impl GithubOAuthConfig {
    fn from_env(request_id: &str) -> Result<Self, ApiError> {
        Ok(Self {
            client_id: required_env("GITHUB_CLIENT_ID", request_id)?,
            client_secret: required_env("GITHUB_CLIENT_SECRET", request_id)?,
            redirect_uri: required_env("GITHUB_OAUTH_REDIRECT_URI", request_id)?,
            success_redirect_path: std::env::var("GITHUB_OAUTH_SUCCESS_REDIRECT_PATH")
                .unwrap_or_else(|_| "/".to_string()),
            authorize_url: std::env::var("GITHUB_OAUTH_AUTHORIZE_URL")
                .unwrap_or_else(|_| DEFAULT_GITHUB_AUTHORIZE_URL.to_string()),
            token_url: std::env::var("GITHUB_OAUTH_TOKEN_URL")
                .unwrap_or_else(|_| DEFAULT_GITHUB_TOKEN_URL.to_string()),
            user_url: std::env::var("GITHUB_OAUTH_USER_URL")
                .unwrap_or_else(|_| DEFAULT_GITHUB_USER_URL.to_string()),
        })
    }
}

fn required_env(name: &str, request_id: &str) -> Result<String, ApiError> {
    std::env::var(name)
        .map_err(|_| ApiError::internal(request_id, format!("missing required env var {name}")))
}

fn github_workspace_slug(login: &str, provider_subject: &str) -> String {
    let normalized_login: String = login
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
        .collect();
    let normalized_login = normalized_login.trim_matches('-');
    let normalized_login = if normalized_login.is_empty() {
        "user"
    } else {
        normalized_login
    };
    let suffix = provider_subject
        .rsplit(':')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("github");

    format!("github-{normalized_login}-{suffix}")
}

fn session_cookie(token: &str) -> String {
    let secure_attr = secure_cookie_attr();
    format!(
        "{SESSION_COOKIE_NAME}={token}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax{}",
        SESSION_TTL_DAYS * 24 * 60 * 60,
        secure_attr
    )
}

fn state_cookie(state: &str) -> String {
    let secure_attr = secure_cookie_attr();
    format!(
        "{GITHUB_OAUTH_STATE_COOKIE_NAME}={state}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax{}",
        GITHUB_OAUTH_STATE_TTL_SECONDS,
        secure_attr
    )
}

fn expired_cookie(name: &str) -> String {
    let secure_attr = secure_cookie_attr();
    format!("{name}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{secure_attr}")
}

fn secure_cookie_attr() -> &'static str {
    if std::env::var("SESSION_COOKIE_SECURE")
        .map(|value| value != "false")
        .unwrap_or(true)
    {
        "; Secure"
    } else {
        ""
    }
}

fn session_cookie_from_headers(headers: &HeaderMap) -> Option<String> {
    cookie_from_headers(headers, SESSION_COOKIE_NAME)
}

fn oauth_state_cookie_from_headers(headers: &HeaderMap) -> Option<String> {
    cookie_from_headers(headers, GITHUB_OAUTH_STATE_COOKIE_NAME)
}

fn cookie_from_headers(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == cookie_name && !value.is_empty()).then(|| value.to_string())
    })
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
    use axum::{
        body::{to_bytes, Body},
        extract::Form,
        extract::State as AxumState,
        http::{Request, StatusCode},
        routing::{get, post},
        Router,
    };
    use serde_json::{json, Value};
    use std::sync::{Mutex, OnceLock};
    use tower::ServiceExt;

    static AUTH_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn auth_env_lock() -> &'static Mutex<()> {
        AUTH_ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    async fn app() -> axum::Router {
        build_test_app(any_test_state().await)
    }

    async fn any_test_state() -> AppState {
        AppState::new(crate::testing::any_test_pool().await, crate::db::DatabaseBackend::Sqlite)
    }

    fn build_test_app(state: AppState) -> axum::Router {
        crate::app::build_router(state)
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

    #[tokio::test]
    async fn github_start_sets_state_cookie_and_redirects() {
        let _guard = auth_env_lock().lock().expect("lock");
        std::env::set_var("GITHUB_CLIENT_ID", "test-client");
        std::env::set_var("GITHUB_CLIENT_SECRET", "test-secret");
        std::env::set_var("GITHUB_OAUTH_REDIRECT_URI", "http://localhost/callback");
        std::env::set_var(
            "GITHUB_OAUTH_AUTHORIZE_URL",
            "https://github.example/login/oauth/authorize",
        );

        let app = app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/github/start")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response
            .headers()
            .get(header::LOCATION)
            .expect("location header")
            .to_str()
            .expect("location");
        assert!(location.contains("client_id=test-client"));
        assert!(location.contains("state="));

        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .expect("set-cookie")
            .to_str()
            .expect("cookie");
        assert!(cookie.starts_with("aw_github_state="));
    }

    #[tokio::test]
    async fn github_callback_rejects_invalid_state() {
        let _guard = auth_env_lock().lock().expect("lock");
        std::env::set_var("GITHUB_CLIENT_ID", "test-client");
        std::env::set_var("GITHUB_CLIENT_SECRET", "test-secret");
        std::env::set_var("GITHUB_OAUTH_REDIRECT_URI", "http://localhost/callback");

        let app = app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/github/callback?code=abc&state=wrong")
                    .header(header::COOKIE, "aw_github_state=expected")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[derive(Clone)]
    struct MockGithubState;

    #[derive(Deserialize)]
    struct MockTokenForm {
        code: String,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    }

    async fn mock_token(
        AxumState(_state): AxumState<MockGithubState>,
        Form(form): Form<MockTokenForm>,
    ) -> Json<Value> {
        assert_eq!(form.code, "good-code");
        assert_eq!(form.client_id, "test-client");
        assert_eq!(form.client_secret, "test-secret");
        assert_eq!(form.redirect_uri, "http://localhost/oauth/callback");
        Json(json!({ "access_token": "gh-token" }))
    }

    async fn mock_user(AxumState(_state): AxumState<MockGithubState>) -> Json<Value> {
        Json(json!({
            "id": 4242,
            "login": "octotest",
            "name": "Octo Test"
        }))
    }

    async fn spawn_mock_github() -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock");
        let addr = listener.local_addr().expect("addr");
        let router = Router::new()
            .route("/login/oauth/access_token", post(mock_token))
            .route("/user", get(mock_user))
            .with_state(MockGithubState);

        let handle = tokio::spawn(async move {
            axum::serve(listener, router).await.expect("serve mock");
        });

        (format!("http://{}", addr), handle)
    }

    #[tokio::test]
    async fn github_callback_creates_session_for_new_identity() {
        let _guard = auth_env_lock().lock().expect("lock");
        let (base_url, handle) = spawn_mock_github().await;
        std::env::set_var("GITHUB_CLIENT_ID", "test-client");
        std::env::set_var("GITHUB_CLIENT_SECRET", "test-secret");
        std::env::set_var("GITHUB_OAUTH_REDIRECT_URI", "http://localhost/oauth/callback");
        std::env::set_var("GITHUB_OAUTH_SUCCESS_REDIRECT_PATH", "/app");
        std::env::set_var(
            "GITHUB_OAUTH_AUTHORIZE_URL",
            format!("{base_url}/login/oauth/authorize"),
        );
        std::env::set_var(
            "GITHUB_OAUTH_TOKEN_URL",
            format!("{base_url}/login/oauth/access_token"),
        );
        std::env::set_var("GITHUB_OAUTH_USER_URL", format!("{base_url}/user"));

        let app = build_test_app(any_test_state().await);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/github/callback?code=good-code&state=match-state")
                    .header(header::COOKIE, "aw_github_state=match-state")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response
                .headers()
                .get(header::LOCATION)
                .expect("location")
                .to_str()
                .expect("location"),
            "/app"
        );

        let session_cookie = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|cookie| cookie.starts_with("aw_session="))
            .expect("session cookie")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string();

        let session_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/session")
                    .header(header::COOKIE, session_cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(session_response.status(), StatusCode::OK);
        let body = body_json(session_response.into_body()).await;
        assert_eq!(body["data"]["authenticated"], true);
        assert_eq!(body["data"]["actor"]["actor_kind"], "human");

        handle.abort();
    }
}

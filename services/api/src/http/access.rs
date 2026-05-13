use crate::http::{
    actor::{ActorContext, ActorKind},
    error::ApiError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkspaceRole {
    Viewer,
    Editor,
    Owner,
}

impl WorkspaceRole {
    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "viewer" => Some(Self::Viewer),
            "editor" => Some(Self::Editor),
            "owner" => Some(Self::Owner),
            _ => None,
        }
    }
}

#[derive(sqlx::FromRow)]
struct MembershipRow {
    role: String,
}

pub fn require_authenticated_human(actor: &ActorContext, request_id: &str) -> Result<(), ApiError> {
    match actor.actor_kind {
        ActorKind::Human => Ok(()),
        ActorKind::System => Err(ApiError::unauthorised(
            request_id,
            "authentication is required",
        )),
        ActorKind::Agent => Err(ApiError::forbidden(
            request_id,
            "agent credentials cannot access this endpoint",
        )),
    }
}

pub async fn require_human_workspace_role(
    pool: &sqlx::AnyPool,
    actor: &ActorContext,
    workspace_id: &str,
    required_role: WorkspaceRole,
    request_id: &str,
) -> Result<(), ApiError> {
    require_authenticated_human(actor, request_id)?;

    let row = sqlx::query_as::<_, MembershipRow>(
        "SELECT target.role AS role
         FROM workspace_members current
         JOIN workspace_members target
           ON target.external_subject = current.external_subject
         WHERE CAST(current.id AS TEXT) = $1
           AND current.status = 'active'
           AND CAST(target.workspace_id AS TEXT) = $2
           AND target.status = 'active'
         LIMIT 1",
    )
    .bind(&actor.actor_id)
    .bind(workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, workspace_id = %workspace_id, "workspace membership lookup failed");
        ApiError::internal(request_id, "failed to resolve workspace membership")
    })?;

    let row = row.ok_or_else(|| {
        ApiError::forbidden(request_id, "actor does not have access to this workspace")
    })?;

    let actual_role = WorkspaceRole::from_db(&row.role).ok_or_else(|| {
        ApiError::internal(request_id, format!("unknown workspace role '{}'", row.role))
    })?;

    if actual_role < required_role {
        return Err(ApiError::forbidden(
            request_id,
            "actor does not have enough permissions for this workspace",
        ));
    }

    Ok(())
}

pub fn require_agent_scope_for_project(
    actor: &ActorContext,
    workspace_id: &str,
    project_id: &str,
    required_scope: &str,
    request_id: &str,
) -> Result<(), ApiError> {
    require_agent_any_scope_for_project(
        actor,
        workspace_id,
        project_id,
        &[required_scope],
        request_id,
    )
}

pub fn require_agent_any_scope_for_project(
    actor: &ActorContext,
    workspace_id: &str,
    project_id: &str,
    required_scopes: &[&str],
    request_id: &str,
) -> Result<(), ApiError> {
    match actor.actor_kind {
        ActorKind::Agent => {}
        ActorKind::System => {
            return Err(ApiError::unauthorised(
                request_id,
                "authentication is required",
            ));
        }
        ActorKind::Human => {
            return Err(ApiError::forbidden(
                request_id,
                "human session cannot use agent-only permissions",
            ));
        }
    }

    if actor.workspace_id.as_deref() != Some(workspace_id) {
        return Err(ApiError::forbidden(
            request_id,
            "agent credential is outside the target workspace",
        ));
    }

    if let Some(bound_project_id) = actor.project_id.as_deref() {
        if bound_project_id != project_id {
            return Err(ApiError::forbidden(
                request_id,
                "agent credential is outside the target project",
            ));
        }
    }

    if !actor
        .scopes
        .iter()
        .any(|scope| required_scopes.iter().any(|required| scope == required))
    {
        let expected = required_scopes.join("' or '");
        return Err(ApiError::forbidden(
            request_id,
            format!("missing required scope '{expected}'"),
        ));
    }

    Ok(())
}

pub async fn require_project_access(
    pool: &sqlx::AnyPool,
    actor: &ActorContext,
    workspace_id: &str,
    project_id: &str,
    human_role: WorkspaceRole,
    agent_scope: Option<&str>,
    request_id: &str,
) -> Result<(), ApiError> {
    match actor.actor_kind {
        ActorKind::Human => {
            require_human_project_role(
                pool,
                actor,
                workspace_id,
                project_id,
                human_role,
                request_id,
            )
            .await
        }
        ActorKind::Agent => {
            let scope = agent_scope.ok_or_else(|| {
                ApiError::forbidden(request_id, "agent credentials cannot access this endpoint")
            })?;
            require_agent_scope_for_project(actor, workspace_id, project_id, scope, request_id)
        }
        ActorKind::System => Err(ApiError::unauthorised(
            request_id,
            "authentication is required",
        )),
    }
}

pub async fn require_project_access_any_agent_scope(
    pool: &sqlx::AnyPool,
    actor: &ActorContext,
    workspace_id: &str,
    project_id: &str,
    human_role: WorkspaceRole,
    agent_scopes: &[&str],
    request_id: &str,
) -> Result<(), ApiError> {
    match actor.actor_kind {
        ActorKind::Human => {
            require_human_project_role(
                pool,
                actor,
                workspace_id,
                project_id,
                human_role,
                request_id,
            )
            .await
        }
        ActorKind::Agent => require_agent_any_scope_for_project(
            actor,
            workspace_id,
            project_id,
            agent_scopes,
            request_id,
        ),
        ActorKind::System => Err(ApiError::unauthorised(
            request_id,
            "authentication is required",
        )),
    }
}

async fn require_human_project_role(
    pool: &sqlx::AnyPool,
    actor: &ActorContext,
    workspace_id: &str,
    project_id: &str,
    required_role: WorkspaceRole,
    request_id: &str,
) -> Result<(), ApiError> {
    require_authenticated_human(actor, request_id)?;

    let row = sqlx::query_as::<_, MembershipRow>(
        "SELECT target.role AS role
         FROM workspace_members current
         JOIN workspace_members target
           ON target.external_subject = current.external_subject
         WHERE CAST(current.id AS TEXT) = $1
           AND current.status = 'active'
           AND CAST(target.workspace_id AS TEXT) = $2
           AND target.status = 'active'
         LIMIT 1",
    )
    .bind(&actor.actor_id)
    .bind(workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, workspace_id = %workspace_id, "workspace membership lookup failed");
        ApiError::internal(request_id, "failed to resolve workspace membership")
    })?;

    let workspace_role = row
        .as_ref()
        .and_then(|row| WorkspaceRole::from_db(&row.role))
        .ok_or_else(|| {
            ApiError::forbidden(request_id, "actor does not have access to this workspace")
        })?;

    if workspace_role == WorkspaceRole::Owner {
        return Ok(());
    }

    let row = sqlx::query_as::<_, MembershipRow>(
        "SELECT pm.role AS role
         FROM workspace_members current
         JOIN workspace_members target
           ON target.external_subject = current.external_subject
         JOIN project_members pm
           ON pm.workspace_member_id = target.id
         WHERE CAST(current.id AS TEXT) = $1
           AND current.status = 'active'
           AND CAST(target.workspace_id AS TEXT) = $2
           AND target.status = 'active'
           AND CAST(pm.project_id AS TEXT) = $3
           AND pm.status = 'active'
         LIMIT 1",
    )
    .bind(&actor.actor_id)
    .bind(workspace_id)
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, workspace_id = %workspace_id, project_id = %project_id, "project membership lookup failed");
        ApiError::internal(request_id, "failed to resolve project membership")
    })?;

    let project_role = row
        .as_ref()
        .and_then(|row| WorkspaceRole::from_db(&row.role))
        .ok_or_else(|| {
            ApiError::forbidden(request_id, "actor does not have access to this project")
        })?;

    if project_role < required_role {
        return Err(ApiError::forbidden(
            request_id,
            "actor does not have enough permissions for this project",
        ));
    }

    Ok(())
}

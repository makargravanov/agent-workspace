use std::convert::Infallible;

use axum::{extract::FromRequestParts, http::request::Parts};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

// TODO BL-10: replace with real auth session/bearer extraction
impl<S> FromRequestParts<S> for ActorContext
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
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
            .map(|s| s.to_string())
            .unwrap_or_else(|| "anonymous".to_string());

        Ok(ActorContext { actor_kind, actor_id })
    }
}

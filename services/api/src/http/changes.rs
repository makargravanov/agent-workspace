use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::Instant;

use crate::{
    http::error::ApiError,
    state::{AppState, ChangeLookup},
};

const DEFAULT_TIMEOUT_MS: u64 = 25_000;
const MIN_TIMEOUT_MS: u64 = 1_000;
const MAX_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, Deserialize)]
pub struct ChangePollQuery {
    pub cursor: Option<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ChangePollResponse {
    pub changed: bool,
    pub cursor: String,
}

pub async fn wait_for_project_change(
    state: &AppState,
    query: ChangePollQuery,
    workspace_id: &str,
    project_id: &str,
    resource_kind: &str,
    request_id: &str,
) -> Result<ChangePollResponse, ApiError> {
    let cursor = match query.cursor.as_deref().map(str::trim) {
        None | Some("") => {
            return Ok(ChangePollResponse {
                changed: false,
                cursor: state.change_notifier.current_cursor().to_string(),
            });
        }
        Some(raw) => raw.parse::<u64>().map_err(|_| {
            ApiError::validation_error(request_id, "cursor must be an unsigned integer")
        })?,
    };

    let timeout_ms = query
        .timeout_ms
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut receiver = state.change_notifier.subscribe();

    loop {
        match state.change_notifier.matching_change_after(
            cursor,
            workspace_id,
            project_id,
            resource_kind,
        ) {
            ChangeLookup::Changed(version) | ChangeLookup::HistoryGap(version) => {
                return Ok(ChangePollResponse {
                    changed: true,
                    cursor: version.to_string(),
                });
            }
            ChangeLookup::Unchanged => {}
        }

        let now = Instant::now();
        if now >= deadline {
            return Ok(ChangePollResponse {
                changed: false,
                cursor: state.change_notifier.current_cursor().to_string(),
            });
        }

        if tokio::time::timeout_at(deadline, receiver.changed())
            .await
            .is_err()
        {
            return Ok(ChangePollResponse {
                changed: false,
                cursor: state.change_notifier.current_cursor().to_string(),
            });
        }
    }
}

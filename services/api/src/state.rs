use crate::db::DatabaseBackend;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tokio::sync::watch;

const CHANGE_HISTORY_LIMIT: usize = 1024;

#[derive(Debug, Clone)]
pub struct ProjectChange {
    pub version: u64,
    pub workspace_id: String,
    pub project_id: String,
    pub resource_kind: String,
}

#[derive(Clone)]
pub struct ChangeNotifier {
    version: Arc<AtomicU64>,
    history: Arc<Mutex<VecDeque<ProjectChange>>>,
    sender: watch::Sender<u64>,
}

impl ChangeNotifier {
    pub fn new() -> Self {
        let (sender, _) = watch::channel(0);
        Self {
            version: Arc::new(AtomicU64::new(0)),
            history: Arc::new(Mutex::new(VecDeque::with_capacity(CHANGE_HISTORY_LIMIT))),
            sender,
        }
    }

    pub fn current_cursor(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.sender.subscribe()
    }

    pub fn publish_project_change(
        &self,
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        resource_kind: impl Into<String>,
    ) -> u64 {
        let version = self.version.fetch_add(1, Ordering::SeqCst) + 1;
        let change = ProjectChange {
            version,
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            resource_kind: resource_kind.into(),
        };

        {
            let mut history = self.history.lock().expect("change history lock poisoned");
            if history.len() == CHANGE_HISTORY_LIMIT {
                history.pop_front();
            }
            history.push_back(change);
        }

        let _ = self.sender.send(version);
        version
    }

    pub fn matching_change_after(
        &self,
        cursor: u64,
        workspace_id: &str,
        project_id: &str,
        resource_kind: &str,
    ) -> ChangeLookup {
        let history = self.history.lock().expect("change history lock poisoned");
        if history
            .front()
            .map(|change| cursor > 0 && change.version > cursor)
            .unwrap_or(false)
        {
            return ChangeLookup::HistoryGap(self.current_cursor());
        }

        history
            .iter()
            .find(|change| {
                change.version > cursor
                    && change.workspace_id == workspace_id
                    && change.project_id == project_id
                    && change.resource_kind == resource_kind
            })
            .map(|change| ChangeLookup::Changed(change.version))
            .unwrap_or(ChangeLookup::Unchanged)
    }
}

impl Default for ChangeNotifier {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeLookup {
    Changed(u64),
    HistoryGap(u64),
    Unchanged,
}

/// Shared application state injected into every handler via `axum::extract::State`.
///
/// The runtime uses `sqlx::AnyPool` so the same handler code can run against
/// PostgreSQL and SQLite.
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::AnyPool,
    pub db_backend: DatabaseBackend,
    pub asset_storage_dir: PathBuf,
    pub change_notifier: ChangeNotifier,
}

impl AppState {
    pub fn new(pool: sqlx::AnyPool, db_backend: DatabaseBackend) -> Self {
        let asset_storage_dir = std::env::var("ASSET_STORAGE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("agent-workspace-assets"));

        Self::new_with_asset_storage(pool, db_backend, asset_storage_dir)
    }

    pub fn new_with_asset_storage(
        pool: sqlx::AnyPool,
        db_backend: DatabaseBackend,
        asset_storage_dir: PathBuf,
    ) -> Self {
        Self {
            pool,
            db_backend,
            asset_storage_dir,
            change_notifier: ChangeNotifier::new(),
        }
    }
}

#[cfg(test)]
impl AppState {
    /// Build a lazy SQLite-backed AnyPool for tests that validate inputs before
    /// any real query is executed.
    pub fn new_lazy_for_test() -> Self {
        sqlx::any::install_default_drivers();

        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1)
            .connect_lazy("sqlite::memory:")
            .expect("lazy AnyPool creation should not fail at URL-parse time");

        Self {
            pool,
            db_backend: DatabaseBackend::Sqlite,
            asset_storage_dir: std::env::temp_dir().join("agent-workspace-assets"),
            change_notifier: ChangeNotifier::new(),
        }
    }
}

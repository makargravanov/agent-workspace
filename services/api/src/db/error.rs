#[derive(Debug)]
pub enum DbError {
    SqlxError(sqlx::Error),
    MigrationError(sqlx::migrate::MigrateError),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::SqlxError(e) => write!(f, "database error: {e}"),
            DbError::MigrationError(e) => write!(f, "migration error: {e}"),
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::SqlxError(e) => Some(e),
            DbError::MigrationError(e) => Some(e),
        }
    }
}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        DbError::SqlxError(e)
    }
}

impl From<sqlx::migrate::MigrateError> for DbError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        DbError::MigrationError(e)
    }
}

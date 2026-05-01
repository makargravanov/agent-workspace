use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
    Postgres,
    Sqlite,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub backend: DatabaseBackend,
}

#[derive(Debug)]
pub enum ConfigError {
    MissingDatabaseUrl,
    InvalidMaxConnections(String),
    InvalidMinConnections(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingDatabaseUrl => {
                write!(f, "DATABASE_URL environment variable is not set")
            }
            ConfigError::InvalidMaxConnections(v) => {
                write!(f, "DATABASE_MAX_CONNECTIONS is not a valid u32: {v}")
            }
            ConfigError::InvalidMinConnections(v) => {
                write!(f, "DATABASE_MIN_CONNECTIONS is not a valid u32: {v}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let url = env::var("DATABASE_URL").map_err(|_| ConfigError::MissingDatabaseUrl)?;

        let max_connections = match env::var("DATABASE_MAX_CONNECTIONS") {
            Ok(v) => v
                .parse::<u32>()
                .map_err(|_| ConfigError::InvalidMaxConnections(v))?,
            Err(_) => 10,
        };

        let min_connections = match env::var("DATABASE_MIN_CONNECTIONS") {
            Ok(v) => v
                .parse::<u32>()
                .map_err(|_| ConfigError::InvalidMinConnections(v))?,
            Err(_) => 1,
        };

        Ok(Self {
            backend: if url.starts_with("sqlite") {
                DatabaseBackend::Sqlite
            } else {
                DatabaseBackend::Postgres
            },
            url,
            max_connections,
            min_connections,
        })
    }
}

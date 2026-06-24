use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use std::{error::Error, fmt, sync::Arc};

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 8081;
const DEFAULT_COORDINATOR_URL: &str = "http://coordinator:8080";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeConfig {
    pub node_id: String,
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub coordinator_url: String,
}

impl NodeConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_getter(|key| std::env::var(key).ok())
    }

    pub fn from_getter<F>(get: F) -> Result<Self, ConfigError>
    where
        F: Fn(&'static str) -> Option<String>,
    {
        let node_id = required("NODE_ID", &get)?;
        let host = get("TSS_NODE_HOST").unwrap_or_else(|| DEFAULT_HOST.to_string());
        let port = parse_port("TSS_NODE_PORT", get("TSS_NODE_PORT"), DEFAULT_PORT)?;
        let database_url = required("DATABASE_URL", &get)?;
        let coordinator_url = trim_trailing_slash(
            get("COORDINATOR_URL").unwrap_or_else(|| DEFAULT_COORDINATOR_URL.to_string()),
        );

        Ok(Self {
            node_id,
            host,
            port,
            database_url,
            coordinator_url,
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    MissingVariable(&'static str),
    InvalidPort {
        variable: &'static str,
        value: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingVariable(variable) => {
                write!(f, "missing required environment variable {variable}")
            }
            Self::InvalidPort { variable, value } => {
                write!(f, "{variable} must be a valid port, got {value}")
            }
        }
    }
}

impl Error for ConfigError {}

#[derive(Clone)]
struct AppState {
    config: Arc<NodeConfig>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    service: &'static str,
    node_id: String,
    status: &'static str,
    database_configured: bool,
    coordinator_url: String,
}

pub fn router(config: NodeConfig) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(AppState {
            config: Arc::new(config),
        })
}

pub async fn run(config: NodeConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bind_address = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!(%bind_address, node_id = %config.node_id, "tss node listening");
    axum::serve(listener, router(config)).await?;
    Ok(())
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "tss-node",
        node_id: state.config.node_id.clone(),
        status: "ok",
        database_configured: !state.config.database_url.is_empty(),
        coordinator_url: state.config.coordinator_url.clone(),
    })
}

fn required<F>(variable: &'static str, get: &F) -> Result<String, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    get(variable)
        .filter(|value| !value.trim().is_empty())
        .ok_or(ConfigError::MissingVariable(variable))
}

fn parse_port(
    variable: &'static str,
    value: Option<String>,
    default: u16,
) -> Result<u16, ConfigError> {
    match value {
        Some(value) => value
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidPort { variable, value }),
        None => Ok(default),
    }
}

fn trim_trailing_slash(value: String) -> String {
    value.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn loads_node_config_with_defaults() {
        let values = HashMap::from([
            ("NODE_ID", "node-a"),
            (
                "DATABASE_URL",
                "postgres://frost:frost@localhost:5432/frost",
            ),
        ]);

        let config = NodeConfig::from_getter(|key| values.get(key).map(|value| value.to_string()))
            .expect("config should load");

        assert_eq!(config.node_id, "node-a");
        assert_eq!(config.host, DEFAULT_HOST);
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.coordinator_url, DEFAULT_COORDINATOR_URL);
    }

    #[test]
    fn requires_node_id() {
        let values = HashMap::from([(
            "DATABASE_URL",
            "postgres://frost:frost@localhost:5432/frost",
        )]);

        let error = NodeConfig::from_getter(|key| values.get(key).map(|value| value.to_string()))
            .expect_err("missing node id should fail");

        assert_eq!(error, ConfigError::MissingVariable("NODE_ID"));
    }
}

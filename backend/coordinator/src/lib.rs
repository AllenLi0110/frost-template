use axum::{extract::State, routing::get, Json, Router};
use reqwest::Client;
use serde::Serialize;
use std::{error::Error, fmt, sync::Arc, time::Duration};

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_SOLANA_RPC_URL: &str = "https://api.devnet.solana.com";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub solana_rpc_url: String,
    pub node_a_url: String,
    pub node_b_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_getter(|key| std::env::var(key).ok())
    }

    pub fn from_getter<F>(get: F) -> Result<Self, ConfigError>
    where
        F: Fn(&'static str) -> Option<String>,
    {
        let host = get("COORDINATOR_HOST").unwrap_or_else(|| DEFAULT_HOST.to_string());
        let port = parse_port("COORDINATOR_PORT", get("COORDINATOR_PORT"), DEFAULT_PORT)?;
        let database_url = required("DATABASE_URL", &get)?;
        let solana_rpc_url =
            get("SOLANA_RPC_URL").unwrap_or_else(|| DEFAULT_SOLANA_RPC_URL.to_string());
        let node_a_url = trim_trailing_slash(required("NODE_A_URL", &get)?);
        let node_b_url = trim_trailing_slash(required("NODE_B_URL", &get)?);

        Ok(Self {
            host,
            port,
            database_url,
            solana_rpc_url,
            node_a_url,
            node_b_url,
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
    config: Arc<AppConfig>,
    http_client: Client,
}

#[derive(Serialize)]
pub struct HealthResponse {
    service: &'static str,
    status: &'static str,
    database_configured: bool,
    solana_rpc_url: String,
    node_a_url: String,
    node_b_url: String,
}

#[derive(Serialize)]
pub struct NodeHealthResponse {
    nodes: Vec<NodeHealth>,
}

#[derive(Serialize)]
pub struct NodeHealth {
    node_id: &'static str,
    url: String,
    reachable: bool,
}

pub fn router(config: AppConfig) -> Router {
    let http_client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("reqwest client should build");
    let state = AppState {
        config: Arc::new(config),
        http_client,
    };

    Router::new()
        .route("/health", get(health))
        .route("/health/nodes", get(node_health))
        .with_state(state)
}

pub async fn run(config: AppConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bind_address = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!(%bind_address, "coordinator listening");
    axum::serve(listener, router(config)).await?;
    Ok(())
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "coordinator",
        status: "ok",
        database_configured: !state.config.database_url.is_empty(),
        solana_rpc_url: state.config.solana_rpc_url.clone(),
        node_a_url: state.config.node_a_url.clone(),
        node_b_url: state.config.node_b_url.clone(),
    })
}

async fn node_health(State(state): State<AppState>) -> Json<NodeHealthResponse> {
    let node_a = check_node(&state.http_client, "node-a", &state.config.node_a_url).await;
    let node_b = check_node(&state.http_client, "node-b", &state.config.node_b_url).await;

    Json(NodeHealthResponse {
        nodes: vec![node_a, node_b],
    })
}

async fn check_node(client: &Client, node_id: &'static str, base_url: &str) -> NodeHealth {
    let health_url = format!("{base_url}/health");
    let reachable = match client.get(&health_url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    };

    NodeHealth {
        node_id,
        url: base_url.to_string(),
        reachable,
    }
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
    fn loads_config_with_defaults() {
        let values = HashMap::from([
            (
                "DATABASE_URL",
                "postgres://frost:frost@localhost:5432/frost",
            ),
            ("NODE_A_URL", "http://localhost:8081/"),
            ("NODE_B_URL", "http://localhost:8082"),
        ]);

        let config = AppConfig::from_getter(|key| values.get(key).map(|value| value.to_string()))
            .expect("config should load");

        assert_eq!(config.host, DEFAULT_HOST);
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.solana_rpc_url, DEFAULT_SOLANA_RPC_URL);
        assert_eq!(config.node_a_url, "http://localhost:8081");
        assert_eq!(config.node_b_url, "http://localhost:8082");
    }

    #[test]
    fn rejects_invalid_port() {
        let values = HashMap::from([
            (
                "DATABASE_URL",
                "postgres://frost:frost@localhost:5432/frost",
            ),
            ("NODE_A_URL", "http://localhost:8081"),
            ("NODE_B_URL", "http://localhost:8082"),
            ("COORDINATOR_PORT", "not-a-port"),
        ]);

        let error = AppConfig::from_getter(|key| values.get(key).map(|value| value.to_string()))
            .expect_err("invalid port should fail");

        assert_eq!(
            error,
            ConfigError::InvalidPort {
                variable: "COORDINATOR_PORT",
                value: "not-a-port".to_string()
            }
        );
    }
}

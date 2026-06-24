use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{error::Error, fmt, sync::Arc};
use uuid::Uuid;

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
    crypto_service: Arc<dyn DkgCryptoService>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    service: &'static str,
    node_id: String,
    status: &'static str,
    database_configured: bool,
    coordinator_url: String,
}

#[derive(Serialize)]
pub struct DkgRoundResponse {
    session_id: Uuid,
    node_id: String,
    round: i32,
    status: &'static str,
    public_payload: Value,
}

pub trait DkgCryptoService: Send + Sync + 'static {
    fn run_dkg_round(&self, config: &NodeConfig, session_id: Uuid, round: i32) -> DkgRoundResponse;
}

#[derive(Clone)]
pub struct PlaceholderDkgCryptoService;

impl DkgCryptoService for PlaceholderDkgCryptoService {
    fn run_dkg_round(&self, config: &NodeConfig, session_id: Uuid, round: i32) -> DkgRoundResponse {
        DkgRoundResponse {
            session_id,
            node_id: config.node_id.clone(),
            round,
            status: "COMPLETED",
            public_payload: json!({
                "kind": "phase-2-placeholder-dkg-round",
                "session_id": session_id,
                "node_id": config.node_id.clone(),
                "round": round
            }),
        }
    }
}

pub fn router(config: NodeConfig) -> Router {
    router_with_crypto_service(config, Arc::new(PlaceholderDkgCryptoService))
}

pub fn router_with_crypto_service(
    config: NodeConfig,
    crypto_service: Arc<dyn DkgCryptoService>,
) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/internal/dkg/{session_id}/round1", post(dkg_round1))
        .route("/internal/dkg/{session_id}/round2", post(dkg_round2))
        .route("/internal/dkg/{session_id}/round3", post(dkg_round3))
        .with_state(AppState {
            config: Arc::new(config),
            crypto_service,
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

async fn dkg_round1(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Json<DkgRoundResponse> {
    Json(run_dkg_round(state, session_id, 1))
}

async fn dkg_round2(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Json<DkgRoundResponse> {
    Json(run_dkg_round(state, session_id, 2))
}

async fn dkg_round3(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Json<DkgRoundResponse> {
    Json(run_dkg_round(state, session_id, 3))
}

fn run_dkg_round(state: AppState, session_id: Uuid, round: i32) -> DkgRoundResponse {
    state
        .crypto_service
        .run_dkg_round(&state.config, session_id, round)
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

    #[test]
    fn placeholder_dkg_round_response_exposes_only_public_payload() {
        let config = NodeConfig {
            node_id: "node-a".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8081,
            database_url: "postgres://frost:frost@localhost:5432/frost".to_string(),
            coordinator_url: "http://localhost:8080".to_string(),
        };
        let session_id = Uuid::new_v4();

        let response = PlaceholderDkgCryptoService.run_dkg_round(&config, session_id, 1);
        let encoded = serde_json::to_value(response).expect("response should serialize");

        assert_eq!(encoded["status"], "COMPLETED");
        assert_eq!(
            encoded["public_payload"]["kind"],
            "phase-2-placeholder-dkg-round"
        );
        assert!(encoded.get("root_share").is_none());
        assert!(encoded.get("nonce_secret").is_none());
        assert!(encoded["public_payload"].get("root_share").is_none());
        assert!(encoded["public_payload"].get("nonce_secret").is_none());
    }
}

use aes_gcm_siv::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256GcmSiv, Nonce,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use frost_ed25519 as frost;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, types::Json as SqlxJson, FromRow, PgPool};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    future::Future,
    pin::Pin,
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 8081;
const DEFAULT_COORDINATOR_URL: &str = "http://coordinator:8080";

const FROST_PACKAGE_FORMAT: &str = "frost-ed25519-2.2.0-hex";
const MAX_SIGNERS: u16 = 2;
const MIN_SIGNERS: u16 = 2;
const STATUS_COMPLETED: &str = "COMPLETED";
const NODE_DKG_STATUS_ROUND_1_COMPLETE: &str = "ROUND_1_COMPLETE";
const NODE_DKG_STATUS_ROUND_2_COMPLETE: &str = "ROUND_2_COMPLETE";
const NODE_DKG_STATUS_COMPLETED: &str = "COMPLETED";
const NODE_SIGNING_STATUS_ROUND_1_COMPLETE: &str = "ROUND_1_COMPLETE";
const NODE_SIGNING_STATUS_ROUND_2_IN_PROGRESS: &str = "ROUND_2_IN_PROGRESS";
const NODE_SIGNING_STATUS_ROUND_2_COMPLETE: &str = "ROUND_2_COMPLETE";
const SIGNING_MESSAGE_FORMAT: &str = "frost-template-transfer-intent-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeConfig {
    pub node_id: String,
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub coordinator_url: String,
    pub node_sealing_key: String,
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
        let node_sealing_key = required("NODE_SEALING_KEY", &get)?;

        Ok(Self {
            node_id,
            host,
            port,
            database_url,
            coordinator_url,
            node_sealing_key,
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
    db_pool: Option<PgPool>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    service: &'static str,
    node_id: String,
    status: &'static str,
    database_configured: bool,
    coordinator_url: String,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct DkgRoundRequest {
    #[serde(default)]
    peer_round1_packages: BTreeMap<String, String>,
    #[serde(default)]
    peer_round2_packages: BTreeMap<String, String>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DkgRoundResponse {
    session_id: Uuid,
    node_id: String,
    round: i32,
    status: &'static str,
    public_payload: Value,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub struct SigningRoundRequest {
    dkg_session_id: Uuid,
    wallet_index: i32,
    sender_address_base58: String,
    recipient_address_base58: String,
    amount_lamports: i64,
    #[serde(default)]
    message_payload: Value,
    #[serde(default)]
    message_hash_hex: String,
    #[serde(default)]
    signing_commitments: BTreeMap<String, String>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct SigningRoundResponse {
    request_id: Uuid,
    node_id: String,
    round: i32,
    status: &'static str,
    public_payload: Value,
}

#[derive(Debug)]
pub enum NodeDkgError {
    DatabaseUnavailable,
    InvalidNode(String),
    InvalidRequest(String),
    MissingPrerequisite(String),
    Crypto(String),
    Database(sqlx::Error),
}

impl fmt::Display for NodeDkgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatabaseUnavailable => write!(f, "database pool is not configured"),
            Self::InvalidNode(node_id) => write!(f, "unsupported node id {node_id}"),
            Self::InvalidRequest(message) => write!(f, "{message}"),
            Self::MissingPrerequisite(message) => write!(f, "{message}"),
            Self::Crypto(message) => write!(f, "{message}"),
            Self::Database(error) => write!(f, "database error: {error}"),
        }
    }
}

impl Error for NodeDkgError {}

impl From<sqlx::Error> for NodeDkgError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl IntoResponse for NodeDkgError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::DatabaseUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::InvalidNode(_) | Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::MissingPrerequisite(_) => StatusCode::CONFLICT,
            Self::Crypto(_) | Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(json!({ "error": self.to_string() }));

        (status, body).into_response()
    }
}

pub struct DkgRoundContext<'a> {
    pub config: &'a NodeConfig,
    pub db_pool: Option<&'a PgPool>,
    pub session_id: Uuid,
    pub round: i32,
    pub request: DkgRoundRequest,
}

type DkgRoundFuture<'a> =
    Pin<Box<dyn Future<Output = Result<DkgRoundResponse, NodeDkgError>> + Send + 'a>>;

pub trait DkgCryptoService: Send + Sync + 'static {
    fn run_dkg_round<'a>(&'a self, context: DkgRoundContext<'a>) -> DkgRoundFuture<'a>;
}

#[derive(Clone)]
pub struct FrostDkgCryptoService;

impl DkgCryptoService for FrostDkgCryptoService {
    fn run_dkg_round<'a>(&'a self, context: DkgRoundContext<'a>) -> DkgRoundFuture<'a> {
        Box::pin(async move {
            match context.round {
                1 => run_frost_dkg_round1(context).await,
                2 => run_frost_dkg_round2(context).await,
                3 => run_frost_dkg_round3(context).await,
                round => Err(NodeDkgError::InvalidRequest(format!(
                    "unsupported DKG round {round}"
                ))),
            }
        })
    }
}

pub fn router(config: NodeConfig) -> Router {
    router_with_pool_and_crypto_service(config, None, Arc::new(FrostDkgCryptoService))
}

pub fn router_with_pool_and_crypto_service(
    config: NodeConfig,
    db_pool: Option<PgPool>,
    crypto_service: Arc<dyn DkgCryptoService>,
) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/internal/dkg/{session_id}/round1", post(dkg_round1))
        .route("/internal/dkg/{session_id}/round2", post(dkg_round2))
        .route("/internal/dkg/{session_id}/round3", post(dkg_round3))
        .route(
            "/internal/signing/{request_id}/round1",
            post(signing_round1),
        )
        .route(
            "/internal/signing/{request_id}/round2",
            post(signing_round2),
        )
        .with_state(AppState {
            config: Arc::new(config),
            crypto_service,
            db_pool,
        })
}

pub async fn run(config: NodeConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    let bind_address = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!(%bind_address, node_id = %config.node_id, "tss node listening");
    axum::serve(
        listener,
        router_with_pool_and_crypto_service(config, Some(db_pool), Arc::new(FrostDkgCryptoService)),
    )
    .await?;
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
    Json(request): Json<DkgRoundRequest>,
) -> Result<Json<DkgRoundResponse>, NodeDkgError> {
    Ok(Json(run_dkg_round(state, session_id, 1, request).await?))
}

async fn dkg_round2(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(request): Json<DkgRoundRequest>,
) -> Result<Json<DkgRoundResponse>, NodeDkgError> {
    Ok(Json(run_dkg_round(state, session_id, 2, request).await?))
}

async fn dkg_round3(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(request): Json<DkgRoundRequest>,
) -> Result<Json<DkgRoundResponse>, NodeDkgError> {
    Ok(Json(run_dkg_round(state, session_id, 3, request).await?))
}

async fn signing_round1(
    State(state): State<AppState>,
    Path(request_id): Path<Uuid>,
    Json(request): Json<SigningRoundRequest>,
) -> Result<Json<SigningRoundResponse>, NodeDkgError> {
    Ok(Json(run_frost_signing_round1(state, request_id, request).await?))
}

async fn signing_round2(
    State(state): State<AppState>,
    Path(request_id): Path<Uuid>,
    Json(request): Json<SigningRoundRequest>,
) -> Result<Json<SigningRoundResponse>, NodeDkgError> {
    Ok(Json(run_frost_signing_round2(state, request_id, request).await?))
}

async fn run_dkg_round(
    state: AppState,
    session_id: Uuid,
    round: i32,
    request: DkgRoundRequest,
) -> Result<DkgRoundResponse, NodeDkgError> {
    state
        .crypto_service
        .run_dkg_round(DkgRoundContext {
            config: &state.config,
            db_pool: state.db_pool.as_ref(),
            session_id,
            round,
            request,
        })
        .await
}

async fn run_frost_dkg_round1(
    context: DkgRoundContext<'_>,
) -> Result<DkgRoundResponse, NodeDkgError> {
    let pool = context.db_pool.ok_or(NodeDkgError::DatabaseUnavailable)?;
    let schema = node_schema(&context.config.node_id)?;

    if let Some(row) = fetch_node_dkg_state(pool, schema, context.session_id).await? {
        if let Some(public_package_hex) = row.round1_public_package_hex {
            return Ok(round1_response(
                context.config,
                context.session_id,
                public_package_hex,
            ));
        }
    }

    let participant_identifier = node_identifier(&context.config.node_id)?;
    let mut rng = OsRng;
    let (round1_secret_package, round1_package) =
        frost::keys::dkg::part1(participant_identifier, MAX_SIGNERS, MIN_SIGNERS, &mut rng)
            .map_err(crypto_error)?;
    let round1_secret_package_ciphertext =
        seal_bytes(context.config, &round1_secret_package.serialize().map_err(crypto_error)?)?;
    let round1_public_package_hex = hex::encode(round1_package.serialize().map_err(crypto_error)?);

    upsert_round1_state(
        pool,
        schema,
        context.session_id,
        &context.config.node_id,
        &round1_secret_package_ciphertext,
        &round1_public_package_hex,
    )
    .await?;

    Ok(round1_response(
        context.config,
        context.session_id,
        round1_public_package_hex,
    ))
}

async fn run_frost_dkg_round2(
    context: DkgRoundContext<'_>,
) -> Result<DkgRoundResponse, NodeDkgError> {
    let pool = context.db_pool.ok_or(NodeDkgError::DatabaseUnavailable)?;
    let schema = node_schema(&context.config.node_id)?;
    let state = fetch_node_dkg_state(pool, schema, context.session_id)
        .await?
        .ok_or_else(|| {
            NodeDkgError::MissingPrerequisite(
                "round 2 requires local round 1 state to be completed".to_string(),
            )
        })?;

    if let Some(round2_public_packages) = state.round2_public_packages {
        return Ok(round2_response(
            context.config,
            context.session_id,
            round2_public_packages.0,
        ));
    }

    let round1_secret_package_ciphertext =
        state.round1_secret_package_ciphertext.ok_or_else(|| {
            NodeDkgError::MissingPrerequisite(
                "round 2 requires local round 1 secret package".to_string(),
            )
        })?;
    let round1_secret_package_bytes =
        open_bytes(context.config, &round1_secret_package_ciphertext)?;
    let round1_secret_package =
        frost::keys::dkg::round1::SecretPackage::deserialize(&round1_secret_package_bytes)
            .map_err(crypto_error)?;
    let peer_round1_packages =
        decode_round1_packages(&context.request.peer_round1_packages, &context.config.node_id)?;

    let (round2_secret_package, round2_packages) =
        frost::keys::dkg::part2(round1_secret_package, &peer_round1_packages)
            .map_err(crypto_error)?;
    let round2_secret_package_ciphertext =
        seal_bytes(context.config, &round2_secret_package.serialize().map_err(crypto_error)?)?;
    let public_packages = encode_round2_packages(round2_packages)?;

    update_round2_state(
        pool,
        schema,
        context.session_id,
        &round2_secret_package_ciphertext,
        &public_packages,
    )
    .await?;

    Ok(round2_response(
        context.config,
        context.session_id,
        public_packages,
    ))
}

async fn run_frost_dkg_round3(
    context: DkgRoundContext<'_>,
) -> Result<DkgRoundResponse, NodeDkgError> {
    let pool = context.db_pool.ok_or(NodeDkgError::DatabaseUnavailable)?;
    let schema = node_schema(&context.config.node_id)?;
    let state = fetch_node_dkg_state(pool, schema, context.session_id)
        .await?
        .ok_or_else(|| {
            NodeDkgError::MissingPrerequisite(
                "round 3 requires local round 2 state to be completed".to_string(),
            )
        })?;

    if let (Some(public_key_package_hex), Some(master_public_key_base58)) = (
        state.public_key_package_hex,
        state.master_public_key_base58,
    ) {
        return Ok(round3_response(
            context.config,
            context.session_id,
            public_key_package_hex,
            master_public_key_base58,
        ));
    }

    let round2_secret_package_ciphertext =
        state.round2_secret_package_ciphertext.ok_or_else(|| {
            NodeDkgError::MissingPrerequisite(
                "round 3 requires local round 2 secret package".to_string(),
            )
        })?;
    let round2_secret_package_bytes =
        open_bytes(context.config, &round2_secret_package_ciphertext)?;
    let round2_secret_package =
        frost::keys::dkg::round2::SecretPackage::deserialize(&round2_secret_package_bytes)
            .map_err(crypto_error)?;
    let peer_round1_packages =
        decode_round1_packages(&context.request.peer_round1_packages, &context.config.node_id)?;
    let peer_round2_packages =
        decode_round2_packages(&context.request.peer_round2_packages, &context.config.node_id)?;

    let (key_package, public_key_package) = frost::keys::dkg::part3(
        &round2_secret_package,
        &peer_round1_packages,
        &peer_round2_packages,
    )
    .map_err(crypto_error)?;
    let key_package_ciphertext =
        seal_bytes(context.config, &key_package.serialize().map_err(crypto_error)?)?;
    let public_key_package_hex = hex::encode(public_key_package.serialize().map_err(crypto_error)?);
    let master_public_key_base58 = master_public_key_base58(&public_key_package)?;

    update_round3_state(
        pool,
        schema,
        context.session_id,
        &key_package_ciphertext,
        &public_key_package_hex,
        &master_public_key_base58,
    )
    .await?;

    Ok(round3_response(
        context.config,
        context.session_id,
        public_key_package_hex,
        master_public_key_base58,
    ))
}

async fn run_frost_signing_round1(
    state: AppState,
    request_id: Uuid,
    request: SigningRoundRequest,
) -> Result<SigningRoundResponse, NodeDkgError> {
    let pool = state.db_pool.as_ref().ok_or(NodeDkgError::DatabaseUnavailable)?;
    let schema = node_schema(&state.config.node_id)?;

    if let Some(row) = fetch_node_signing_state(pool, schema, request_id).await? {
        if let Some(commitment_payload) = row.commitment_payload {
            return Ok(signing_round1_response(
                &state.config,
                request_id,
                request.wallet_index,
                commitment_payload.0,
            ));
        }
    }

    let key_package =
        load_completed_key_package(pool, schema, &state.config, request.dkg_session_id).await?;
    let mut rng = OsRng;
    let (signing_nonces, signing_commitments) =
        frost::round1::commit(key_package.signing_share(), &mut rng);
    let signing_nonces_ciphertext =
        seal_bytes(&state.config, &signing_nonces.serialize().map_err(crypto_error)?)?;
    let commitments_hex = hex::encode(signing_commitments.serialize().map_err(crypto_error)?);
    let commitment_payload = signing_round1_payload(
        &state.config,
        request_id,
        request.wallet_index,
        commitments_hex,
    );

    upsert_signing_round1_state(
        pool,
        schema,
        request_id,
        &state.config.node_id,
        request.wallet_index,
        &signing_nonces_ciphertext,
        &commitment_payload,
    )
    .await?;

    Ok(signing_round1_response(
        &state.config,
        request_id,
        request.wallet_index,
        commitment_payload,
    ))
}

async fn run_frost_signing_round2(
    state: AppState,
    request_id: Uuid,
    request: SigningRoundRequest,
) -> Result<SigningRoundResponse, NodeDkgError> {
    let pool = state.db_pool.as_ref().ok_or(NodeDkgError::DatabaseUnavailable)?;
    let schema = node_schema(&state.config.node_id)?;
    let signing_state = fetch_node_signing_state(pool, schema, request_id)
        .await?
        .ok_or_else(|| {
            NodeDkgError::MissingPrerequisite(
                "signing round 2 requires local round 1 nonce state".to_string(),
            )
        })?;

    if signing_state.round2_consumed_at.is_some() || signing_state.signature_share_hex.is_some() {
        return Err(NodeDkgError::MissingPrerequisite(
            "signing round 2 nonce has already been consumed".to_string(),
        ));
    }

    let signing_nonces_ciphertext = signing_state.signing_nonces_ciphertext.ok_or_else(|| {
        NodeDkgError::MissingPrerequisite(
            "signing round 2 requires encrypted local nonce state".to_string(),
        )
    })?;
    let signing_nonces_bytes = open_bytes(&state.config, &signing_nonces_ciphertext)?;
    let signing_nonces =
        frost::round1::SigningNonces::deserialize(&signing_nonces_bytes).map_err(crypto_error)?;
    let key_package =
        load_completed_key_package(pool, schema, &state.config, request.dkg_session_id).await?;
    let message_bytes = signing_message_bytes(&request)?;
    let commitments = decode_signing_commitments(&request.signing_commitments, &state.config.node_id)?;
    claim_signing_nonce_for_round2(pool, schema, request_id, &request.message_hash_hex).await?;
    let signing_package = frost::SigningPackage::new(commitments, &message_bytes);
    let signature_share =
        frost::round2::sign(&signing_package, &signing_nonces, &key_package).map_err(crypto_error)?;
    let signature_share_hex = hex::encode(signature_share.serialize());

    store_signing_round2_share(pool, schema, request_id, &signature_share_hex).await?;

    Ok(signing_round2_response(
        &state.config,
        request_id,
        request.wallet_index,
        request.message_hash_hex,
        signature_share_hex,
    ))
}

fn round1_response(
    config: &NodeConfig,
    session_id: Uuid,
    public_package_hex: String,
) -> DkgRoundResponse {
    DkgRoundResponse {
        session_id,
        node_id: config.node_id.clone(),
        round: 1,
        status: STATUS_COMPLETED,
        public_payload: json!({
            "kind": "frost-dkg-round1",
            "package_format": FROST_PACKAGE_FORMAT,
            "session_id": session_id,
            "node_id": config.node_id.clone(),
            "round": 1,
            "public_package_hex": public_package_hex
        }),
    }
}

fn round2_response(
    config: &NodeConfig,
    session_id: Uuid,
    round2_packages: Value,
) -> DkgRoundResponse {
    DkgRoundResponse {
        session_id,
        node_id: config.node_id.clone(),
        round: 2,
        status: STATUS_COMPLETED,
        public_payload: json!({
            "kind": "frost-dkg-round2",
            "package_format": FROST_PACKAGE_FORMAT,
            "session_id": session_id,
            "node_id": config.node_id.clone(),
            "round": 2,
            "round2_packages": round2_packages
        }),
    }
}

fn round3_response(
    config: &NodeConfig,
    session_id: Uuid,
    public_key_package_hex: String,
    master_public_key_base58: String,
) -> DkgRoundResponse {
    DkgRoundResponse {
        session_id,
        node_id: config.node_id.clone(),
        round: 3,
        status: STATUS_COMPLETED,
        public_payload: json!({
            "kind": "frost-dkg-round3",
            "package_format": FROST_PACKAGE_FORMAT,
            "session_id": session_id,
            "node_id": config.node_id.clone(),
            "round": 3,
            "public_key_package_hex": public_key_package_hex,
            "master_public_key_base58": master_public_key_base58
        }),
    }
}

fn signing_round1_response(
    config: &NodeConfig,
    request_id: Uuid,
    wallet_index: i32,
    public_payload: Value,
) -> SigningRoundResponse {
    SigningRoundResponse {
        request_id,
        node_id: config.node_id.clone(),
        round: 1,
        status: STATUS_COMPLETED,
        public_payload: if public_payload.is_null() {
            signing_round1_payload(config, request_id, wallet_index, String::new())
        } else {
            public_payload
        },
    }
}

fn signing_round1_payload(
    config: &NodeConfig,
    request_id: Uuid,
    wallet_index: i32,
    commitments_hex: String,
) -> Value {
    json!({
        "kind": "frost-signing-round1",
        "package_format": FROST_PACKAGE_FORMAT,
        "request_id": request_id,
        "node_id": config.node_id.clone(),
        "round": 1,
        "wallet_index": wallet_index,
        "commitment_scope": "root-key-package-transfer-intent",
        "commitments_hex": commitments_hex
    })
}

fn signing_round2_response(
    config: &NodeConfig,
    request_id: Uuid,
    wallet_index: i32,
    message_hash_hex: String,
    signature_share_hex: String,
) -> SigningRoundResponse {
    SigningRoundResponse {
        request_id,
        node_id: config.node_id.clone(),
        round: 2,
        status: STATUS_COMPLETED,
        public_payload: json!({
            "kind": "frost-signing-round2",
            "package_format": FROST_PACKAGE_FORMAT,
            "request_id": request_id,
            "node_id": config.node_id.clone(),
            "round": 2,
            "wallet_index": wallet_index,
            "signature_scope": "root-key-package-transfer-intent",
            "message_hash_hex": message_hash_hex,
            "signature_share_hex": signature_share_hex
        }),
    }
}

#[derive(Debug, FromRow)]
struct NodeDkgStateRow {
    round1_secret_package_ciphertext: Option<String>,
    round1_public_package_hex: Option<String>,
    round2_secret_package_ciphertext: Option<String>,
    round2_public_packages: Option<SqlxJson<Value>>,
    public_key_package_hex: Option<String>,
    master_public_key_base58: Option<String>,
}

#[derive(Debug, FromRow)]
struct NodeSigningStateRow {
    commitment_payload: Option<SqlxJson<Value>>,
    signing_nonces_ciphertext: Option<String>,
    signature_share_hex: Option<String>,
    round2_consumed_at: Option<String>,
}

#[derive(Debug, FromRow)]
struct CompletedKeyPackageRow {
    key_package_ciphertext: String,
}

async fn fetch_node_dkg_state(
    pool: &PgPool,
    schema: &str,
    session_id: Uuid,
) -> Result<Option<NodeDkgStateRow>, NodeDkgError> {
    let query = format!(
        r#"
        SELECT
            round1_secret_package_ciphertext,
            round1_public_package_hex,
            round2_secret_package_ciphertext,
            round2_public_packages,
            public_key_package_hex,
            master_public_key_base58
        FROM {schema}.node_dkg_state
        WHERE session_id = $1
        "#
    );

    sqlx::query_as::<_, NodeDkgStateRow>(&query)
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .map_err(NodeDkgError::from)
}

async fn fetch_node_signing_state(
    pool: &PgPool,
    schema: &str,
    request_id: Uuid,
) -> Result<Option<NodeSigningStateRow>, NodeDkgError> {
    let query = format!(
        r#"
        SELECT
            commitment_payload,
            signing_nonces_ciphertext,
            signature_share_hex,
            round2_consumed_at::text AS round2_consumed_at
        FROM {schema}.node_signing_states
        WHERE request_id = $1
        "#
    );

    sqlx::query_as::<_, NodeSigningStateRow>(&query)
        .bind(request_id)
        .fetch_optional(pool)
        .await
        .map_err(NodeDkgError::from)
}

async fn load_completed_key_package(
    pool: &PgPool,
    schema: &str,
    config: &NodeConfig,
    dkg_session_id: Uuid,
) -> Result<frost::keys::KeyPackage, NodeDkgError> {
    let query = format!(
        r#"
        SELECT key_package_ciphertext
        FROM {schema}.node_dkg_state
        WHERE status = $1 AND session_id = $2 AND key_package_ciphertext IS NOT NULL
        "#
    );
    let row = sqlx::query_as::<_, CompletedKeyPackageRow>(&query)
        .bind(NODE_DKG_STATUS_COMPLETED)
        .bind(dkg_session_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| {
            NodeDkgError::MissingPrerequisite(
                "signing requires a completed local FROST DKG key package".to_string(),
            )
        })?;
    let key_package_bytes = open_bytes(config, &row.key_package_ciphertext)?;

    frost::keys::KeyPackage::deserialize(&key_package_bytes).map_err(crypto_error)
}

async fn upsert_round1_state(
    pool: &PgPool,
    schema: &str,
    session_id: Uuid,
    node_id: &str,
    round1_secret_package_ciphertext: &str,
    round1_public_package_hex: &str,
) -> Result<(), NodeDkgError> {
    let query = format!(
        r#"
        INSERT INTO {schema}.node_dkg_state
            (session_id, node_id, status, round1_secret_package_ciphertext, round1_public_package_hex)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (session_id) DO UPDATE
        SET status = EXCLUDED.status,
            round1_secret_package_ciphertext = EXCLUDED.round1_secret_package_ciphertext,
            round1_public_package_hex = EXCLUDED.round1_public_package_hex,
            updated_at = now()
        "#
    );

    sqlx::query(&query)
        .bind(session_id)
        .bind(node_id)
        .bind(NODE_DKG_STATUS_ROUND_1_COMPLETE)
        .bind(round1_secret_package_ciphertext)
        .bind(round1_public_package_hex)
        .execute(pool)
        .await?;

    Ok(())
}

async fn upsert_signing_round1_state(
    pool: &PgPool,
    schema: &str,
    request_id: Uuid,
    node_id: &str,
    wallet_index: i32,
    signing_nonces_ciphertext: &str,
    commitment_payload: &Value,
) -> Result<(), NodeDkgError> {
    let query = format!(
        r#"
        INSERT INTO {schema}.node_signing_states
            (request_id, node_id, wallet_index, status, signing_nonces_ciphertext, commitment_payload)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (request_id) DO UPDATE
        SET status = EXCLUDED.status,
            signing_nonces_ciphertext = COALESCE({schema}.node_signing_states.signing_nonces_ciphertext, EXCLUDED.signing_nonces_ciphertext),
            commitment_payload = COALESCE({schema}.node_signing_states.commitment_payload, EXCLUDED.commitment_payload),
            updated_at = now()
        WHERE {schema}.node_signing_states.round2_consumed_at IS NULL
        "#
    );

    sqlx::query(&query)
        .bind(request_id)
        .bind(node_id)
        .bind(wallet_index)
        .bind(NODE_SIGNING_STATUS_ROUND_1_COMPLETE)
        .bind(signing_nonces_ciphertext)
        .bind(SqlxJson(commitment_payload.clone()))
        .execute(pool)
        .await?;

    Ok(())
}

async fn update_round2_state(
    pool: &PgPool,
    schema: &str,
    session_id: Uuid,
    round2_secret_package_ciphertext: &str,
    round2_public_packages: &Value,
) -> Result<(), NodeDkgError> {
    let query = format!(
        r#"
        UPDATE {schema}.node_dkg_state
        SET status = $2,
            round2_secret_package_ciphertext = $3,
            round2_public_packages = $4,
            updated_at = now()
        WHERE session_id = $1
        "#
    );

    let result = sqlx::query(&query)
        .bind(session_id)
        .bind(NODE_DKG_STATUS_ROUND_2_COMPLETE)
        .bind(round2_secret_package_ciphertext)
        .bind(SqlxJson(round2_public_packages.clone()))
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(NodeDkgError::MissingPrerequisite(
            "round 2 requires local round 1 state to be completed".to_string(),
        ));
    }

    Ok(())
}

async fn claim_signing_nonce_for_round2(
    pool: &PgPool,
    schema: &str,
    request_id: Uuid,
    message_hash_hex: &str,
) -> Result<(), NodeDkgError> {
    let query = format!(
        r#"
        UPDATE {schema}.node_signing_states
        SET status = $2,
            message_hash_hex = $3,
            round2_consumed_at = now(),
            updated_at = now()
        WHERE request_id = $1
          AND round2_consumed_at IS NULL
          AND signature_share_hex IS NULL
        "#
    );

    let result = sqlx::query(&query)
        .bind(request_id)
        .bind(NODE_SIGNING_STATUS_ROUND_2_IN_PROGRESS)
        .bind(message_hash_hex)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(NodeDkgError::MissingPrerequisite(
            "signing round 2 nonce has already been consumed".to_string(),
        ));
    }

    Ok(())
}

async fn store_signing_round2_share(
    pool: &PgPool,
    schema: &str,
    request_id: Uuid,
    signature_share_hex: &str,
) -> Result<(), NodeDkgError> {
    let query = format!(
        r#"
        UPDATE {schema}.node_signing_states
        SET status = $2,
            signature_share_hex = $3,
            updated_at = now()
        WHERE request_id = $1
          AND round2_consumed_at IS NOT NULL
          AND signature_share_hex IS NULL
        "#
    );

    let result = sqlx::query(&query)
        .bind(request_id)
        .bind(NODE_SIGNING_STATUS_ROUND_2_COMPLETE)
        .bind(signature_share_hex)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(NodeDkgError::MissingPrerequisite(
            "signing round 2 nonce has already been consumed".to_string(),
        ));
    }

    Ok(())
}

async fn update_round3_state(
    pool: &PgPool,
    schema: &str,
    session_id: Uuid,
    key_package_ciphertext: &str,
    public_key_package_hex: &str,
    master_public_key_base58: &str,
) -> Result<(), NodeDkgError> {
    let query = format!(
        r#"
        UPDATE {schema}.node_dkg_state
        SET status = $2,
            key_package_ciphertext = $3,
            public_key_package_hex = $4,
            master_public_key_base58 = $5,
            updated_at = now()
        WHERE session_id = $1
        "#
    );

    let result = sqlx::query(&query)
        .bind(session_id)
        .bind(NODE_DKG_STATUS_COMPLETED)
        .bind(key_package_ciphertext)
        .bind(public_key_package_hex)
        .bind(master_public_key_base58)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(NodeDkgError::MissingPrerequisite(
            "round 3 requires local round 2 state to be completed".to_string(),
        ));
    }

    Ok(())
}

fn decode_round1_packages(
    packages: &BTreeMap<String, String>,
    current_node_id: &str,
) -> Result<BTreeMap<frost::Identifier, frost::keys::dkg::round1::Package>, NodeDkgError> {
    if packages.len() != 1 {
        return Err(NodeDkgError::MissingPrerequisite(
            "2-of-2 DKG requires exactly one peer round 1 package".to_string(),
        ));
    }

    packages
        .iter()
        .map(|(node_id, package_hex)| {
            validate_peer_node_id(node_id, current_node_id)?;
            let package_bytes = decode_hex_field("peer round 1 package", package_hex)?;
            let package =
                frost::keys::dkg::round1::Package::deserialize(&package_bytes).map_err(crypto_error)?;

            Ok((node_identifier(node_id)?, package))
        })
        .collect()
}

fn decode_round2_packages(
    packages: &BTreeMap<String, String>,
    current_node_id: &str,
) -> Result<BTreeMap<frost::Identifier, frost::keys::dkg::round2::Package>, NodeDkgError> {
    if packages.len() != 1 {
        return Err(NodeDkgError::MissingPrerequisite(
            "2-of-2 DKG requires exactly one peer round 2 package".to_string(),
        ));
    }

    packages
        .iter()
        .map(|(node_id, package_hex)| {
            validate_peer_node_id(node_id, current_node_id)?;
            let package_bytes = decode_hex_field("peer round 2 package", package_hex)?;
            let package =
                frost::keys::dkg::round2::Package::deserialize(&package_bytes).map_err(crypto_error)?;

            Ok((node_identifier(node_id)?, package))
        })
        .collect()
}

fn encode_round2_packages(
    packages: BTreeMap<frost::Identifier, frost::keys::dkg::round2::Package>,
) -> Result<Value, NodeDkgError> {
    let mut encoded = serde_json::Map::new();

    for (identifier, package) in packages {
        let node_id = node_id_for_identifier(identifier)?;
        encoded.insert(
            node_id.to_string(),
            Value::String(hex::encode(package.serialize().map_err(crypto_error)?)),
        );
    }

    Ok(Value::Object(encoded))
}

fn decode_signing_commitments(
    commitments: &BTreeMap<String, String>,
    current_node_id: &str,
) -> Result<BTreeMap<frost::Identifier, frost::round1::SigningCommitments>, NodeDkgError> {
    if commitments.len() != 2 {
        return Err(NodeDkgError::MissingPrerequisite(
            "2-of-2 signing requires commitments from both nodes".to_string(),
        ));
    }

    if !commitments.contains_key(current_node_id) {
        return Err(NodeDkgError::MissingPrerequisite(
            "signing commitments must include the current node".to_string(),
        ));
    }

    commitments
        .iter()
        .map(|(node_id, commitments_hex)| {
            node_schema(node_id)?;
            let commitment_bytes = decode_hex_field("signing commitments", commitments_hex)?;
            let signing_commitments =
                frost::round1::SigningCommitments::deserialize(&commitment_bytes)
                    .map_err(crypto_error)?;

            Ok((node_identifier(node_id)?, signing_commitments))
        })
        .collect()
}

fn signing_message_bytes(request: &SigningRoundRequest) -> Result<Vec<u8>, NodeDkgError> {
    if request.message_payload.get("format").and_then(Value::as_str)
        != Some(SIGNING_MESSAGE_FORMAT)
    {
        return Err(NodeDkgError::InvalidRequest(
            "signing message payload has an unsupported format".to_string(),
        ));
    }

    let canonical_message = request
        .message_payload
        .get("canonical_message")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            NodeDkgError::InvalidRequest(
                "signing message payload is missing canonical_message".to_string(),
            )
        })?;
    let mut hasher = Sha256::new();
    hasher.update(canonical_message.as_bytes());
    let computed_hash = hex::encode(hasher.finalize());

    if computed_hash != request.message_hash_hex {
        return Err(NodeDkgError::InvalidRequest(
            "signing message hash does not match canonical_message".to_string(),
        ));
    }

    Ok(canonical_message.as_bytes().to_vec())
}

fn decode_hex_field(field: &str, value: &str) -> Result<Vec<u8>, NodeDkgError> {
    hex::decode(value).map_err(|error| {
        NodeDkgError::InvalidRequest(format!("{field} must be lowercase hex: {error}"))
    })
}

fn master_public_key_base58(
    public_key_package: &frost::keys::PublicKeyPackage,
) -> Result<String, NodeDkgError> {
    let verifying_key_bytes = public_key_package
        .verifying_key()
        .serialize()
        .map_err(crypto_error)?;

    Ok(bs58::encode(verifying_key_bytes).into_string())
}

fn seal_bytes(config: &NodeConfig, plaintext: &[u8]) -> Result<String, NodeDkgError> {
    let key = derive_sealing_key(config);
    let cipher = Aes256GcmSiv::new_from_slice(&key)
        .map_err(|error| NodeDkgError::Crypto(format!("failed to create node cipher: {error}")))?;
    let mut nonce_bytes = [0_u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|error| NodeDkgError::Crypto(format!("failed to encrypt node material: {error:?}")))?;

    Ok(format!(
        "v1:{}:{}",
        hex::encode(nonce_bytes),
        hex::encode(ciphertext)
    ))
}

fn open_bytes(config: &NodeConfig, sealed: &str) -> Result<Vec<u8>, NodeDkgError> {
    let mut parts = sealed.split(':');
    let version = parts.next();
    let nonce_hex = parts.next();
    let ciphertext_hex = parts.next();

    if version != Some("v1") || nonce_hex.is_none() || ciphertext_hex.is_none() || parts.next().is_some() {
        return Err(NodeDkgError::Crypto(
            "encrypted node material has an unsupported format".to_string(),
        ));
    }

    let nonce_bytes = hex::decode(nonce_hex.expect("nonce should exist"))
        .map_err(|error| NodeDkgError::Crypto(format!("node material nonce is invalid: {error}")))?;
    if nonce_bytes.len() != 12 {
        return Err(NodeDkgError::Crypto(
            "node material nonce must be 12 bytes".to_string(),
        ));
    }
    let ciphertext = hex::decode(ciphertext_hex.expect("ciphertext should exist")).map_err(|error| {
        NodeDkgError::Crypto(format!("node material ciphertext is invalid: {error}"))
    })?;
    let key = derive_sealing_key(config);
    let cipher = Aes256GcmSiv::new_from_slice(&key)
        .map_err(|error| NodeDkgError::Crypto(format!("failed to create node cipher: {error}")))?;

    cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|error| NodeDkgError::Crypto(format!("failed to decrypt node material: {error:?}")))
}

fn derive_sealing_key(config: &NodeConfig) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"frost-template-node-dkg-v1");
    hasher.update(config.node_id.as_bytes());
    hasher.update(config.node_sealing_key.as_bytes());
    let digest = hasher.finalize();
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest);
    key
}

fn validate_peer_node_id(node_id: &str, current_node_id: &str) -> Result<(), NodeDkgError> {
    if node_id == current_node_id {
        return Err(NodeDkgError::InvalidRequest(
            "peer package map must not include the current node".to_string(),
        ));
    }

    node_schema(node_id).map(|_| ())
}

fn node_schema(node_id: &str) -> Result<&'static str, NodeDkgError> {
    match node_id {
        "node-a" => Ok("node_a"),
        "node-b" => Ok("node_b"),
        _ => Err(NodeDkgError::InvalidNode(node_id.to_string())),
    }
}

fn node_identifier(node_id: &str) -> Result<frost::Identifier, NodeDkgError> {
    let identifier_index = match node_id {
        "node-a" => 1_u16,
        "node-b" => 2_u16,
        _ => return Err(NodeDkgError::InvalidNode(node_id.to_string())),
    };

    identifier_index.try_into().map_err(crypto_error)
}

fn node_id_for_identifier(identifier: frost::Identifier) -> Result<&'static str, NodeDkgError> {
    if identifier == node_identifier("node-a")? {
        return Ok("node-a");
    }
    if identifier == node_identifier("node-b")? {
        return Ok("node-b");
    }

    Err(NodeDkgError::InvalidRequest(
        "round 2 package was addressed to an unknown node identifier".to_string(),
    ))
}

fn crypto_error(error: impl fmt::Debug) -> NodeDkgError {
    NodeDkgError::Crypto(format!("FROST DKG operation failed: {error:?}"))
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
            ("NODE_SEALING_KEY", "test-node-a-sealing-key"),
        ]);

        let config = NodeConfig::from_getter(|key| values.get(key).map(|value| value.to_string()))
            .expect("config should load");

        assert_eq!(config.node_id, "node-a");
        assert_eq!(config.host, DEFAULT_HOST);
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.coordinator_url, DEFAULT_COORDINATOR_URL);
        assert_eq!(config.node_sealing_key, "test-node-a-sealing-key");
    }

    #[test]
    fn requires_node_sealing_key() {
        let values = HashMap::from([
            ("NODE_ID", "node-a"),
            (
                "DATABASE_URL",
                "postgres://frost:frost@localhost:5432/frost",
            ),
        ]);

        let error = NodeConfig::from_getter(|key| values.get(key).map(|value| value.to_string()))
            .expect_err("missing sealing key should fail");

        assert_eq!(error, ConfigError::MissingVariable("NODE_SEALING_KEY"));
    }

    #[test]
    fn frost_two_of_two_dkg_produces_matching_master_public_key() {
        let mut rng = OsRng;
        let node_a_identifier = node_identifier("node-a").expect("node a id should exist");
        let node_b_identifier = node_identifier("node-b").expect("node b id should exist");

        let (node_a_round1_secret, node_a_round1_package) = frost::keys::dkg::part1(
            node_a_identifier,
            MAX_SIGNERS,
            MIN_SIGNERS,
            &mut rng,
        )
        .expect("node a round 1 should work");
        let (node_b_round1_secret, node_b_round1_package) = frost::keys::dkg::part1(
            node_b_identifier,
            MAX_SIGNERS,
            MIN_SIGNERS,
            &mut rng,
        )
        .expect("node b round 1 should work");

        let node_a_peer_round1 =
            BTreeMap::from([(node_b_identifier, node_b_round1_package.clone())]);
        let node_b_peer_round1 =
            BTreeMap::from([(node_a_identifier, node_a_round1_package.clone())]);

        let (node_a_round2_secret, node_a_round2_packages) =
            frost::keys::dkg::part2(node_a_round1_secret, &node_a_peer_round1)
                .expect("node a round 2 should work");
        let (node_b_round2_secret, node_b_round2_packages) =
            frost::keys::dkg::part2(node_b_round1_secret, &node_b_peer_round1)
                .expect("node b round 2 should work");

        let node_a_peer_round2 = BTreeMap::from([(
            node_b_identifier,
            node_b_round2_packages
                .get(&node_a_identifier)
                .expect("node b should emit package for node a")
                .clone(),
        )]);
        let node_b_peer_round2 = BTreeMap::from([(
            node_a_identifier,
            node_a_round2_packages
                .get(&node_b_identifier)
                .expect("node a should emit package for node b")
                .clone(),
        )]);

        let (node_a_key_package, node_a_public_key_package) = frost::keys::dkg::part3(
            &node_a_round2_secret,
            &node_a_peer_round1,
            &node_a_peer_round2,
        )
        .expect("node a round 3 should work");
        let (node_b_key_package, node_b_public_key_package) = frost::keys::dkg::part3(
            &node_b_round2_secret,
            &node_b_peer_round1,
            &node_b_peer_round2,
        )
        .expect("node b round 3 should work");

        let node_a_master_public_key = master_public_key_base58(&node_a_public_key_package)
            .expect("node a master key should encode");
        let node_b_master_public_key = master_public_key_base58(&node_b_public_key_package)
            .expect("node b master key should encode");

        assert_eq!(node_a_master_public_key, node_b_master_public_key);
        assert!(!node_a_master_public_key.is_empty());
        assert!(!node_a_key_package
            .serialize()
            .expect("node a key package should serialize")
            .is_empty());
        assert!(!node_b_key_package
            .serialize()
            .expect("node b key package should serialize")
            .is_empty());
    }

    #[test]
    fn node_material_encryption_round_trips_without_plaintext() {
        let config = test_config("node-a");
        let plaintext = b"private-root-material";

        let sealed = seal_bytes(&config, plaintext).expect("material should seal");
        let opened = open_bytes(&config, &sealed).expect("material should open");

        assert_eq!(opened, plaintext);
        assert!(sealed.starts_with("v1:"));
        assert!(!sealed.contains("private-root-material"));
    }

    #[test]
    fn frost_public_payloads_do_not_expose_private_field_names() {
        let config = test_config("node-a");
        let session_id = Uuid::new_v4();
        let encoded = serde_json::to_string(&round3_response(
            &config,
            session_id,
            "public-key-package".to_string(),
            "master-public-key".to_string(),
        ))
        .expect("response should serialize");

        for forbidden in [
            "root_share",
            "private_share",
            "nonce_secret",
            "secret_key",
            "key_package_ciphertext",
            "round1_secret_package_ciphertext",
            "round2_secret_package_ciphertext",
        ] {
            assert!(
                !encoded.contains(forbidden),
                "public response must not contain {forbidden}"
            );
        }
    }

    #[test]
    fn signing_message_hash_must_match_canonical_message() {
        let canonical_message = "frost-template-transfer-intent-v1\nrequest_id=test";
        let mut hasher = Sha256::new();
        hasher.update(canonical_message.as_bytes());
        let message_hash_hex = hex::encode(hasher.finalize());
        let request = SigningRoundRequest {
            dkg_session_id: Uuid::new_v4(),
            wallet_index: 0,
            sender_address_base58: bs58::encode([2_u8; 32]).into_string(),
            recipient_address_base58: bs58::encode([3_u8; 32]).into_string(),
            amount_lamports: 1,
            message_payload: json!({
                "format": SIGNING_MESSAGE_FORMAT,
                "canonical_message": canonical_message
            }),
            message_hash_hex,
            signing_commitments: BTreeMap::new(),
        };

        let message_bytes =
            signing_message_bytes(&request).expect("matching hash should validate");

        assert_eq!(message_bytes, canonical_message.as_bytes());

        let tampered = SigningRoundRequest {
            message_hash_hex: "00".repeat(32),
            ..request
        };
        let error =
            signing_message_bytes(&tampered).expect_err("tampered hash should be rejected");

        assert!(matches!(error, NodeDkgError::InvalidRequest(_)));
    }

    #[test]
    fn signing_public_payloads_do_not_expose_nonce_state() {
        let config = test_config("node-a");
        let request_id = Uuid::new_v4();
        let encoded = serde_json::to_string(&vec![
            signing_round1_response(
                &config,
                request_id,
                0,
                signing_round1_payload(&config, request_id, 0, "commitment".to_string()),
            )
            .public_payload,
            signing_round2_response(
                &config,
                request_id,
                0,
                "message-hash".to_string(),
                "signature-share".to_string(),
            )
            .public_payload,
        ])
        .expect("responses should serialize");

        for forbidden in [
            "root_share",
            "private_share",
            "nonce_secret",
            "secret_key",
            "key_package_ciphertext",
            "signing_nonces_ciphertext",
        ] {
            assert!(
                !encoded.contains(forbidden),
                "public signing response must not contain {forbidden}"
            );
        }
    }

    fn test_config(node_id: &str) -> NodeConfig {
        NodeConfig {
            node_id: node_id.to_string(),
            host: "127.0.0.1".to_string(),
            port: 8081,
            database_url: "postgres://frost:frost@localhost:5432/frost".to_string(),
            coordinator_url: "http://localhost:8080".to_string(),
            node_sealing_key: format!("{node_id}-test-sealing-key"),
        }
    }
}

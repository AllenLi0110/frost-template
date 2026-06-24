use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use generic_ec::{curves::Ed25519, Point};
use hd_wallet::{edwards, ExtendedPublicKey, NonHardenedIndex};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, types::Json as SqlxJson, Executor, FromRow, PgPool};
use std::{collections::BTreeMap, error::Error, fmt, sync::Arc, time::Duration};
use uuid::Uuid;

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_SOLANA_RPC_URL: &str = "https://api.devnet.solana.com";

const NODE_IDS: [&str; 2] = ["node-a", "node-b"];
const DKG_ROUNDS: [i32; 3] = [1, 2, 3];
const STATUS_NOT_STARTED: &str = "NOT_STARTED";
const STATUS_COMPLETED: &str = "COMPLETED";
const STATUS_RUNNING: &str = "RUNNING";
const STATUS_FAILED: &str = "FAILED";
const STATUS_ROUND_1_IN_PROGRESS: &str = "ROUND_1_IN_PROGRESS";
const STATUS_ROUND_1_COMPLETE: &str = "ROUND_1_COMPLETE";
const STATUS_ROUND_2_IN_PROGRESS: &str = "ROUND_2_IN_PROGRESS";
const STATUS_ROUND_2_COMPLETE: &str = "ROUND_2_COMPLETE";
const STATUS_ROUND_3_IN_PROGRESS: &str = "ROUND_3_IN_PROGRESS";
const PUBLIC_DERIVATION_SCHEME: &str = "hd-wallet-edwards-v1";
const PUBLIC_DERIVATION_CONTEXT_DOMAIN: &str = "frost-template-public-derivation-context-v1";
const BALANCE_STATUS_AVAILABLE: &str = "AVAILABLE";
const BALANCE_STATUS_UNAVAILABLE: &str = "UNAVAILABLE";
const SIGNING_ROUNDS: [i32; 2] = [1, 2];
const SIGNING_STATUS_PENDING: &str = "PENDING";
const SIGNING_STATUS_COMMITMENTS_IN_PROGRESS: &str = "COMMITMENTS_IN_PROGRESS";
const SIGNING_STATUS_COMMITMENTS_READY: &str = "COMMITMENTS_READY";
const SIGNING_STATUS_SHARES_IN_PROGRESS: &str = "SHARES_IN_PROGRESS";
const SIGNING_STATUS_READY_TO_AGGREGATE: &str = "READY_TO_AGGREGATE";
const SIGNING_MESSAGE_FORMAT: &str = "frost-template-transfer-intent-v1";

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
    db_pool: Option<PgPool>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    service: &'static str,
    status: &'static str,
    database_configured: bool,
    solana_rpc_configured: bool,
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

#[derive(Deserialize)]
pub struct CreateDkgSessionRequest {
    threshold: i32,
    participants: Vec<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DkgSessionResponse {
    session_id: Uuid,
    status: String,
    master_public_key_base58: Option<String>,
    node_steps: Vec<DkgNodeStepResponse>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DkgNodeStepResponse {
    node_id: String,
    round: i32,
    status: String,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct TriggerDkgRoundResponse {
    session_id: Uuid,
    node_id: String,
    round: i32,
    status: String,
    dkg_status: String,
    public_payload: Option<Value>,
}

#[derive(Deserialize)]
struct NodeDkgRoundResponse {
    session_id: Uuid,
    node_id: String,
    round: i32,
    status: String,
    public_payload: Value,
}

#[derive(Serialize, Debug, Default, PartialEq, Eq)]
struct NodeDkgRoundRequest {
    peer_round1_packages: BTreeMap<String, String>,
    peer_round2_packages: BTreeMap<String, String>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct WalletListResponse {
    wallets: Vec<WalletResponse>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct WalletResponse {
    wallet_index: i32,
    dkg_session_id: Uuid,
    derivation_path: String,
    public_key_base58: String,
    address_base58: String,
    balance_lamports: Option<i64>,
    balance_status: String,
    balance_error_message: Option<String>,
    balance_checked_at: Option<String>,
    created_at: String,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct WalletBalanceResponse {
    wallet_index: i32,
    address_base58: String,
    balance_lamports: Option<i64>,
    balance_status: String,
    balance_error_message: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CreateSigningRequestRequest {
    wallet_index: i32,
    recipient_address_base58: String,
    amount_lamports: i64,
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq, Eq)]
struct SigningRequestQuery {
    status: Option<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct SigningRequestListResponse {
    requests: Vec<SigningRequestResponse>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct SigningRequestResponse {
    request_id: Uuid,
    wallet_index: i32,
    sender_address_base58: String,
    recipient_address_base58: String,
    amount_lamports: i64,
    status: String,
    message_hash_hex: Option<String>,
    recent_blockhash: Option<String>,
    transaction_signature: Option<String>,
    explorer_url: Option<String>,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
    node_steps: Vec<SigningNodeStepResponse>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SigningNodeStepResponse {
    node_id: String,
    round: i32,
    status: String,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct TriggerSigningRoundResponse {
    request_id: Uuid,
    node_id: String,
    round: i32,
    status: String,
    signing_status: String,
    public_payload: Option<Value>,
}

#[derive(Deserialize)]
struct NodeSigningRoundResponse {
    request_id: Uuid,
    node_id: String,
    round: i32,
    status: String,
    public_payload: Value,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
struct NodeSigningRoundRequest {
    wallet_index: i32,
    sender_address_base58: String,
    recipient_address_base58: String,
    amount_lamports: i64,
    message_payload: Value,
    message_hash_hex: String,
    signing_commitments: BTreeMap<String, String>,
}

#[derive(Debug, Clone, FromRow)]
struct DkgSessionRow {
    id: Uuid,
    status: String,
    master_public_key_base58: Option<String>,
    public_derivation_context: Option<SqlxJson<Value>>,
}

#[derive(Debug, Clone, FromRow)]
struct DkgStepRow {
    node_id: String,
    round: i32,
    status: String,
    public_payload: Option<SqlxJson<Value>>,
}

#[derive(Debug, Clone, FromRow)]
struct WalletRow {
    wallet_index: i32,
    dkg_session_id: Uuid,
    derivation_path: String,
    public_key_base58: String,
    address_base58: String,
    balance_lamports: Option<i64>,
    balance_status: String,
    balance_error_message: Option<String>,
    balance_checked_at: Option<String>,
    created_at: String,
}

#[derive(Debug, Clone, FromRow)]
struct SigningRequestRow {
    id: Uuid,
    wallet_index: i32,
    sender_address_base58: String,
    recipient_address_base58: String,
    amount_lamports: i64,
    status: String,
    message_payload: Option<SqlxJson<Value>>,
    message_hash_hex: Option<String>,
    recent_blockhash: Option<String>,
    transaction_signature: Option<String>,
    explorer_url: Option<String>,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, FromRow)]
struct SigningStepRow {
    node_id: String,
    round: i32,
    status: String,
    public_payload: Option<SqlxJson<Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DerivedWallet {
    wallet_index: i32,
    derivation_path: String,
    public_key_base58: String,
    address_base58: String,
}

#[derive(Deserialize)]
struct SolanaBalanceRpcResponse {
    result: Option<SolanaBalanceResult>,
    error: Option<SolanaRpcError>,
}

#[derive(Deserialize)]
struct SolanaBalanceResult {
    value: u64,
}

#[derive(Deserialize)]
struct SolanaRpcError {
    message: String,
}

#[derive(Debug)]
enum DkgError {
    DatabaseUnavailable,
    InvalidCreateRequest(String),
    InvalidNode(String),
    InvalidRound(i32),
    InvalidWalletIndex(i32),
    SessionNotFound,
    TransitionBlocked(String),
    NodeCallFailed(String),
    WalletNotFound(i32),
    SigningRequestNotFound,
    InvalidSigningRequest(String),
    InvalidSigningRound(i32),
    SigningTransitionBlocked(String),
    WalletDerivationBlocked(String),
    WalletDerivationFailed(String),
    Database(sqlx::Error),
}

impl fmt::Display for DkgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatabaseUnavailable => write!(f, "database pool is not configured"),
            Self::InvalidCreateRequest(message) => write!(f, "{message}"),
            Self::InvalidNode(node_id) => write!(f, "unsupported node id {node_id}"),
            Self::InvalidRound(round) => write!(f, "unsupported DKG round {round}"),
            Self::InvalidWalletIndex(wallet_index) => {
                write!(f, "unsupported wallet index {wallet_index}")
            }
            Self::SessionNotFound => write!(f, "DKG session not found"),
            Self::TransitionBlocked(message) => write!(f, "{message}"),
            Self::NodeCallFailed(message) => write!(f, "{message}"),
            Self::WalletNotFound(wallet_index) => {
                write!(f, "wallet index {wallet_index} not found")
            }
            Self::SigningRequestNotFound => write!(f, "signing request not found"),
            Self::InvalidSigningRequest(message) => write!(f, "{message}"),
            Self::InvalidSigningRound(round) => write!(f, "unsupported signing round {round}"),
            Self::SigningTransitionBlocked(message) => write!(f, "{message}"),
            Self::WalletDerivationBlocked(message) => write!(f, "{message}"),
            Self::WalletDerivationFailed(message) => write!(f, "{message}"),
            Self::Database(error) => write!(f, "database error: {error}"),
        }
    }
}

impl Error for DkgError {}

impl From<sqlx::Error> for DkgError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl IntoResponse for DkgError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::DatabaseUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::InvalidCreateRequest(_)
            | Self::InvalidNode(_)
            | Self::InvalidRound(_)
            | Self::InvalidWalletIndex(_)
            | Self::InvalidSigningRequest(_)
            | Self::InvalidSigningRound(_) => StatusCode::BAD_REQUEST,
            Self::SessionNotFound | Self::WalletNotFound(_) | Self::SigningRequestNotFound => {
                StatusCode::NOT_FOUND
            }
            Self::TransitionBlocked(_)
            | Self::WalletDerivationBlocked(_)
            | Self::SigningTransitionBlocked(_) => StatusCode::CONFLICT,
            Self::NodeCallFailed(_) => StatusCode::BAD_GATEWAY,
            Self::WalletDerivationFailed(_) | Self::Database(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        let body = Json(json!({ "error": self.to_string() }));

        (status, body).into_response()
    }
}

pub fn router(config: AppConfig) -> Router {
    router_with_pool(config, None)
}

pub fn router_with_pool(config: AppConfig, db_pool: Option<PgPool>) -> Router {
    let http_client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client should build");
    let state = AppState {
        config: Arc::new(config),
        http_client,
        db_pool,
    };

    Router::new()
        .route("/health", get(health))
        .route("/health/nodes", get(node_health))
        .route("/api/dkg/sessions", post(create_dkg_session))
        .route("/api/dkg/sessions/active", get(get_active_dkg_session))
        .route("/api/wallets", get(list_wallets).post(create_wallet))
        .route(
            "/api/signing-requests",
            get(list_signing_requests).post(create_signing_request),
        )
        .route("/api/signing-requests/{request_id}", get(get_signing_request))
        .route(
            "/api/wallets/{wallet_index}/balance",
            get(refresh_wallet_balance),
        )
        .route(
            "/api/signing-requests/{request_id}/nodes/{node_id}/rounds/{round}",
            post(trigger_signing_round),
        )
        .route(
            "/api/dkg/sessions/{session_id}/nodes/{node_id}/rounds/{round}",
            post(trigger_dkg_round),
        )
        .with_state(state)
}

pub async fn run(config: AppConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;
    run_migrations(&db_pool).await?;

    let bind_address = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!(%bind_address, "coordinator listening");
    axum::serve(listener, router_with_pool(config, Some(db_pool))).await?;
    Ok(())
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "coordinator",
        status: "ok",
        database_configured: !state.config.database_url.is_empty(),
        solana_rpc_configured: !state.config.solana_rpc_url.is_empty(),
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

async fn create_wallet(State(state): State<AppState>) -> Result<Json<WalletResponse>, DkgError> {
    let pool = db_pool(&state)?;
    let session = completed_wallet_session(pool).await?;
    let master_public_key = session
        .master_public_key_base58
        .as_deref()
        .ok_or_else(wallet_derivation_prerequisite_error)?;
    let public_derivation_context = ensure_public_derivation_context(pool, &session).await?;
    let mut transaction = pool.begin().await?;

    sqlx::query("LOCK TABLE coordinator.wallets IN EXCLUSIVE MODE")
        .execute(&mut *transaction)
        .await?;

    let wallet_index = sqlx::query_scalar::<_, i32>(
        r#"
        SELECT COALESCE(MAX(wallet_index) + 1, 0)
        FROM coordinator.wallets
        "#,
    )
    .fetch_one(&mut *transaction)
    .await?;
    let derived_wallet =
        derive_wallet(master_public_key, &public_derivation_context, wallet_index)?;
    let wallet = sqlx::query_as::<_, WalletRow>(
        r#"
        INSERT INTO coordinator.wallets
            (wallet_index, dkg_session_id, derivation_path, public_key_base58, address_base58)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
            wallet_index,
            dkg_session_id,
            derivation_path,
            public_key_base58,
            address_base58,
            balance_lamports,
            balance_status,
            balance_error_message,
            balance_checked_at::text AS balance_checked_at,
            created_at::text AS created_at
        "#,
    )
    .bind(derived_wallet.wallet_index)
    .bind(session.id)
    .bind(&derived_wallet.derivation_path)
    .bind(&derived_wallet.public_key_base58)
    .bind(&derived_wallet.address_base58)
    .fetch_one(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(Json(wallet.into()))
}

async fn list_wallets(State(state): State<AppState>) -> Result<Json<WalletListResponse>, DkgError> {
    let pool = db_pool(&state)?;
    let wallets = fetch_wallets(pool).await?;

    Ok(Json(WalletListResponse {
        wallets: wallets.into_iter().map(WalletResponse::from).collect(),
    }))
}

async fn refresh_wallet_balance(
    State(state): State<AppState>,
    Path(wallet_index): Path<i32>,
) -> Result<Json<WalletBalanceResponse>, DkgError> {
    validate_wallet_index(wallet_index)?;
    let pool = db_pool(&state)?;
    let wallet = fetch_wallet(pool, wallet_index)
        .await?
        .ok_or(DkgError::WalletNotFound(wallet_index))?;
    let balance_result = fetch_balance_lamports(
        &state.http_client,
        &state.config.solana_rpc_url,
        &wallet.address_base58,
    )
    .await;

    let (balance_lamports, balance_status, balance_error_message) = match balance_result {
        Ok(balance_lamports) => (
            Some(balance_lamports),
            BALANCE_STATUS_AVAILABLE.to_string(),
            None,
        ),
        Err(error_message) => (
            None,
            BALANCE_STATUS_UNAVAILABLE.to_string(),
            Some(error_message),
        ),
    };

    let updated_wallet = update_wallet_balance(
        pool,
        wallet_index,
        balance_lamports,
        &balance_status,
        balance_error_message.as_deref(),
    )
    .await?;

    Ok(Json(WalletBalanceResponse {
        wallet_index: updated_wallet.wallet_index,
        address_base58: updated_wallet.address_base58,
        balance_lamports: updated_wallet.balance_lamports,
        balance_status: updated_wallet.balance_status,
        balance_error_message: updated_wallet.balance_error_message,
    }))
}

async fn create_signing_request(
    State(state): State<AppState>,
    Json(payload): Json<CreateSigningRequestRequest>,
) -> Result<Json<SigningRequestResponse>, DkgError> {
    validate_create_signing_request(&payload)?;
    let pool = db_pool(&state)?;
    let wallet = fetch_wallet(pool, payload.wallet_index)
        .await?
        .ok_or(DkgError::WalletNotFound(payload.wallet_index))?;
    let request_id = Uuid::new_v4();
    let mut transaction = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO coordinator.signing_requests
            (id, wallet_index, sender_address_base58, recipient_address_base58, amount_lamports, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(request_id)
    .bind(payload.wallet_index)
    .bind(&wallet.address_base58)
    .bind(&payload.recipient_address_base58)
    .bind(payload.amount_lamports)
    .bind(SIGNING_STATUS_PENDING)
    .execute(&mut *transaction)
    .await?;

    for node_id in NODE_IDS {
        for round in SIGNING_ROUNDS {
            sqlx::query(
                r#"
                INSERT INTO coordinator.signing_node_steps
                    (request_id, node_id, round, status)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(request_id)
            .bind(node_id)
            .bind(round)
            .bind(STATUS_NOT_STARTED)
            .execute(&mut *transaction)
            .await?;
        }
    }

    transaction.commit().await?;

    let request = fetch_signing_request(pool, request_id)
        .await?
        .ok_or(DkgError::SigningRequestNotFound)?;

    Ok(Json(fetch_signing_request_response(pool, request).await?))
}

async fn list_signing_requests(
    State(state): State<AppState>,
    Query(query): Query<SigningRequestQuery>,
) -> Result<Json<SigningRequestListResponse>, DkgError> {
    let pool = db_pool(&state)?;
    let requests = fetch_signing_requests(pool, query.status.as_deref()).await?;
    let mut responses = Vec::with_capacity(requests.len());

    for request in requests {
        responses.push(fetch_signing_request_response(pool, request).await?);
    }

    Ok(Json(SigningRequestListResponse { requests: responses }))
}

async fn get_signing_request(
    State(state): State<AppState>,
    Path(request_id): Path<Uuid>,
) -> Result<Json<SigningRequestResponse>, DkgError> {
    let pool = db_pool(&state)?;
    let request = fetch_signing_request(pool, request_id)
        .await?
        .ok_or(DkgError::SigningRequestNotFound)?;

    Ok(Json(fetch_signing_request_response(pool, request).await?))
}

async fn trigger_signing_round(
    State(state): State<AppState>,
    Path((request_id, node_id, round)): Path<(Uuid, String, i32)>,
) -> Result<Json<TriggerSigningRoundResponse>, DkgError> {
    validate_node_id(&node_id)?;
    validate_signing_round(round)?;

    let pool = db_pool(&state)?;
    let request = fetch_signing_request(pool, request_id)
        .await?
        .ok_or(DkgError::SigningRequestNotFound)?;
    let steps = fetch_signing_steps(pool, request_id).await?;
    let step = steps
        .iter()
        .find(|step| step.node_id == node_id && step.round == round)
        .ok_or(DkgError::SigningRequestNotFound)?;

    if step.status == STATUS_COMPLETED {
        if round == 1 {
            return Ok(Json(completed_signing_step_response(&request, step)));
        }

        return Err(DkgError::SigningTransitionBlocked(format!(
            "{node_id} signing round 2 nonce has already been consumed"
        )));
    }

    validate_signing_round_prerequisites(&steps, round)?;

    if claim_signing_step(pool, request_id, &node_id, round)
        .await?
        .is_none()
    {
        let current_request = fetch_signing_request(pool, request_id)
            .await?
            .ok_or(DkgError::SigningRequestNotFound)?;
        let current_step = fetch_signing_step(pool, request_id, &node_id, round)
            .await?
            .ok_or(DkgError::SigningRequestNotFound)?;

        if current_step.status == STATUS_COMPLETED && round == 1 {
            return Ok(Json(completed_signing_step_response(
                &current_request,
                &current_step,
            )));
        }

        if current_step.status == STATUS_COMPLETED && round == 2 {
            return Err(DkgError::SigningTransitionBlocked(format!(
                "{node_id} signing round 2 nonce has already been consumed"
            )));
        }

        return Err(DkgError::SigningTransitionBlocked(format!(
            "{node_id} signing round {round} is already {}",
            current_step.status
        )));
    }

    let node_request =
        build_node_signing_round_request(pool, &request, &steps, &node_id, round).await?;
    let node_response =
        call_node_signing_round(&state, request_id, &node_id, round, &node_request).await;
    let node_response = match node_response {
        Ok(node_response) => node_response,
        Err(error) => {
            mark_signing_step_failed(pool, request_id, &node_id, round, &error.to_string()).await?;
            return Err(error);
        }
    };

    if node_response.request_id != request_id
        || node_response.node_id != node_id
        || node_response.round != round
        || node_response.status != STATUS_COMPLETED
    {
        let error = DkgError::NodeCallFailed(
            "TSS node returned a signing round response that does not match the request"
                .to_string(),
        );
        mark_signing_step_failed(pool, request_id, &node_id, round, &error.to_string()).await?;
        return Err(error);
    }

    sqlx::query(
        r#"
        UPDATE coordinator.signing_node_steps
        SET status = $1, public_payload = $2, error_message = NULL, completed_at = now(), updated_at = now()
        WHERE request_id = $3 AND node_id = $4 AND round = $5
        "#,
    )
    .bind(STATUS_COMPLETED)
    .bind(SqlxJson(node_response.public_payload.clone()))
    .bind(request_id)
    .bind(&node_id)
    .bind(round)
    .execute(pool)
    .await?;

    let updated_steps = fetch_signing_steps(pool, request_id).await?;
    let signing_status = derive_signing_status(&updated_steps);

    sqlx::query(
        r#"
        UPDATE coordinator.signing_requests
        SET status = $1, updated_at = now()
        WHERE id = $2
        "#,
    )
    .bind(&signing_status)
    .bind(request_id)
    .execute(pool)
    .await?;

    Ok(Json(TriggerSigningRoundResponse {
        request_id,
        node_id,
        round,
        status: STATUS_COMPLETED.to_string(),
        signing_status,
        public_payload: Some(node_response.public_payload),
    }))
}

async fn create_dkg_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateDkgSessionRequest>,
) -> Result<Json<DkgSessionResponse>, DkgError> {
    validate_create_request(&payload)?;
    let pool = db_pool(&state)?;

    if let Some(session) = fetch_active_session(pool).await? {
        return Ok(Json(fetch_session_response(pool, session).await?));
    }

    let session_id = Uuid::new_v4();
    let mut transaction = pool.begin().await?;

    let insert_result = sqlx::query(
        r#"
        INSERT INTO coordinator.dkg_sessions
            (id, threshold, participant_count, status, active)
        VALUES ($1, $2, $3, $4, TRUE)
        "#,
    )
    .bind(session_id)
    .bind(payload.threshold)
    .bind(payload.participants.len() as i32)
    .bind(STATUS_NOT_STARTED)
    .execute(&mut *transaction)
    .await;

    if let Err(error) = insert_result {
        let unique_violation = is_unique_violation(&error);
        let _ = transaction.rollback().await;

        if unique_violation {
            if let Some(session) = fetch_active_session(pool).await? {
                return Ok(Json(fetch_session_response(pool, session).await?));
            }
        }

        return Err(DkgError::Database(error));
    }

    for node_id in NODE_IDS {
        for round in DKG_ROUNDS {
            sqlx::query(
                r#"
                INSERT INTO coordinator.dkg_node_steps
                    (session_id, node_id, round, status)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(session_id)
            .bind(node_id)
            .bind(round)
            .bind(STATUS_NOT_STARTED)
            .execute(&mut *transaction)
            .await?;
        }
    }

    transaction.commit().await?;

    let session = fetch_session(pool, session_id)
        .await?
        .ok_or(DkgError::SessionNotFound)?;

    Ok(Json(fetch_session_response(pool, session).await?))
}

async fn get_active_dkg_session(
    State(state): State<AppState>,
) -> Result<Json<DkgSessionResponse>, DkgError> {
    let pool = db_pool(&state)?;
    let session = fetch_active_session(pool)
        .await?
        .ok_or(DkgError::SessionNotFound)?;

    Ok(Json(fetch_session_response(pool, session).await?))
}

async fn trigger_dkg_round(
    State(state): State<AppState>,
    Path((session_id, node_id, round)): Path<(Uuid, String, i32)>,
) -> Result<Json<TriggerDkgRoundResponse>, DkgError> {
    validate_node_id(&node_id)?;
    validate_round(round)?;

    let pool = db_pool(&state)?;
    let session = fetch_session(pool, session_id)
        .await?
        .ok_or(DkgError::SessionNotFound)?;
    let steps = fetch_session_steps(pool, session_id).await?;
    let step = steps
        .iter()
        .find(|step| step.node_id == node_id && step.round == round)
        .ok_or(DkgError::SessionNotFound)?;

    if step.status == STATUS_COMPLETED {
        return Ok(Json(completed_step_response(&session, step)));
    }

    validate_round_prerequisites(&steps, round)?;

    if claim_dkg_step(pool, session_id, &node_id, round)
        .await?
        .is_none()
    {
        let current_session = fetch_session(pool, session_id)
            .await?
            .ok_or(DkgError::SessionNotFound)?;
        let current_step = fetch_session_step(pool, session_id, &node_id, round)
            .await?
            .ok_or(DkgError::SessionNotFound)?;

        if current_step.status == STATUS_COMPLETED {
            return Ok(Json(completed_step_response(
                &current_session,
                &current_step,
            )));
        }

        return Err(DkgError::TransitionBlocked(format!(
            "{node_id} DKG round {round} is already {}",
            current_step.status
        )));
    }

    let node_request = build_node_dkg_round_request(&steps, &node_id, round)?;
    let node_response =
        call_node_dkg_round(&state, session_id, &node_id, round, &node_request).await;
    let node_response = match node_response {
        Ok(node_response) => node_response,
        Err(error) => {
            mark_step_failed(pool, session_id, &node_id, round, &error.to_string()).await?;
            return Err(error);
        }
    };

    if node_response.session_id != session_id
        || node_response.node_id != node_id
        || node_response.round != round
        || node_response.status != STATUS_COMPLETED
    {
        let error = DkgError::NodeCallFailed(
            "TSS node returned a DKG round response that does not match the request".to_string(),
        );
        mark_step_failed(pool, session_id, &node_id, round, &error.to_string()).await?;
        return Err(error);
    }

    sqlx::query(
        r#"
        UPDATE coordinator.dkg_node_steps
        SET status = $1, public_payload = $2, error_message = NULL, completed_at = now(), updated_at = now()
        WHERE session_id = $3 AND node_id = $4 AND round = $5
        "#,
    )
    .bind(STATUS_COMPLETED)
    .bind(SqlxJson(node_response.public_payload.clone()))
    .bind(session_id)
    .bind(&node_id)
    .bind(round)
    .execute(pool)
    .await?;

    let updated_steps = fetch_session_steps(pool, session_id).await?;
    let dkg_status = derive_dkg_status(&updated_steps);
    let master_public_key = if dkg_status == STATUS_COMPLETED {
        Some(extract_completed_master_public_key(&updated_steps)?)
    } else {
        None
    };
    let public_derivation_context = master_public_key
        .as_ref()
        .map(|master_public_key| SqlxJson(build_public_derivation_context(master_public_key)));

    sqlx::query(
        r#"
        UPDATE coordinator.dkg_sessions
        SET status = $1,
            master_public_key_base58 = COALESCE($2, master_public_key_base58),
            public_derivation_context = COALESCE($3, public_derivation_context),
            updated_at = now()
        WHERE id = $4
        "#,
    )
    .bind(&dkg_status)
    .bind(master_public_key)
    .bind(public_derivation_context)
    .bind(session_id)
    .execute(pool)
    .await?;

    Ok(Json(TriggerDkgRoundResponse {
        session_id,
        node_id,
        round,
        status: STATUS_COMPLETED.to_string(),
        dkg_status,
        public_payload: Some(client_public_payload(&node_response.public_payload)),
    }))
}

async fn call_node_dkg_round(
    state: &AppState,
    session_id: Uuid,
    node_id: &str,
    round: i32,
    request: &NodeDkgRoundRequest,
) -> Result<NodeDkgRoundResponse, DkgError> {
    let node_url = match node_id {
        "node-a" => &state.config.node_a_url,
        "node-b" => &state.config.node_b_url,
        _ => return Err(DkgError::InvalidNode(node_id.to_string())),
    };
    let url = format!("{node_url}/internal/dkg/{session_id}/round{round}");
    let response = state
        .http_client
        .post(&url)
        .json(request)
        .send()
        .await
        .map_err(|error| DkgError::NodeCallFailed(format!("{node_id} DKG call failed: {error}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "".to_string());
        return Err(DkgError::NodeCallFailed(format!(
            "{node_id} DKG call returned {status}: {body}"
        )));
    }

    response
        .json::<NodeDkgRoundResponse>()
        .await
        .map_err(|error| {
            DkgError::NodeCallFailed(format!("{node_id} DKG response was invalid: {error}"))
        })
}

async fn call_node_signing_round(
    state: &AppState,
    request_id: Uuid,
    node_id: &str,
    round: i32,
    request: &NodeSigningRoundRequest,
) -> Result<NodeSigningRoundResponse, DkgError> {
    let node_url = match node_id {
        "node-a" => &state.config.node_a_url,
        "node-b" => &state.config.node_b_url,
        _ => return Err(DkgError::InvalidNode(node_id.to_string())),
    };
    let url = format!("{node_url}/internal/signing/{request_id}/round{round}");
    let response = state
        .http_client
        .post(&url)
        .json(request)
        .send()
        .await
        .map_err(|error| {
            DkgError::NodeCallFailed(format!("{node_id} signing call failed: {error}"))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "".to_string());
        return Err(DkgError::NodeCallFailed(format!(
            "{node_id} signing call returned {status}: {body}"
        )));
    }

    response
        .json::<NodeSigningRoundResponse>()
        .await
        .map_err(|error| {
            DkgError::NodeCallFailed(format!("{node_id} signing response was invalid: {error}"))
        })
}

async fn mark_step_failed(
    pool: &PgPool,
    session_id: Uuid,
    node_id: &str,
    round: i32,
    error_message: &str,
) -> Result<(), DkgError> {
    sqlx::query(
        r#"
        UPDATE coordinator.dkg_node_steps
        SET status = $1, error_message = $2, updated_at = now()
        WHERE session_id = $3 AND node_id = $4 AND round = $5
        "#,
    )
    .bind(STATUS_FAILED)
    .bind(error_message)
    .bind(session_id)
    .bind(node_id)
    .bind(round)
    .execute(pool)
    .await?;

    Ok(())
}

async fn mark_signing_step_failed(
    pool: &PgPool,
    request_id: Uuid,
    node_id: &str,
    round: i32,
    error_message: &str,
) -> Result<(), DkgError> {
    sqlx::query(
        r#"
        UPDATE coordinator.signing_node_steps
        SET status = $1, error_message = $2, updated_at = now()
        WHERE request_id = $3 AND node_id = $4 AND round = $5
        "#,
    )
    .bind(STATUS_FAILED)
    .bind(error_message)
    .bind(request_id)
    .bind(node_id)
    .bind(round)
    .execute(pool)
    .await?;

    Ok(())
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

async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    run_sql_file(
        pool,
        include_str!("../../migrations/0001_create_foundation_schemas.sql"),
    )
    .await?;
    run_sql_file(
        pool,
        include_str!("../../migrations/0002_create_dkg_tables.sql"),
    )
    .await?;
    run_sql_file(
        pool,
        include_str!("../../migrations/0003_create_node_dkg_state.sql"),
    )
    .await?;
    run_sql_file(
        pool,
        include_str!("../../migrations/0004_create_wallet_tables.sql"),
    )
    .await?;
    run_sql_file(
        pool,
        include_str!("../../migrations/0005_create_signing_tables.sql"),
    )
    .await
}

async fn claim_dkg_step(
    pool: &PgPool,
    session_id: Uuid,
    node_id: &str,
    round: i32,
) -> Result<Option<DkgStepRow>, DkgError> {
    sqlx::query_as::<_, DkgStepRow>(
        r#"
        UPDATE coordinator.dkg_node_steps
        SET status = $1, error_message = NULL, updated_at = now()
        WHERE session_id = $2
          AND node_id = $3
          AND round = $4
          AND status IN ($5, $6)
        RETURNING node_id, round, status, public_payload
        "#,
    )
    .bind(STATUS_RUNNING)
    .bind(session_id)
    .bind(node_id)
    .bind(round)
    .bind(STATUS_NOT_STARTED)
    .bind(STATUS_FAILED)
    .fetch_optional(pool)
    .await
    .map_err(DkgError::from)
}

async fn claim_signing_step(
    pool: &PgPool,
    request_id: Uuid,
    node_id: &str,
    round: i32,
) -> Result<Option<SigningStepRow>, DkgError> {
    sqlx::query_as::<_, SigningStepRow>(
        r#"
        UPDATE coordinator.signing_node_steps
        SET status = $1, error_message = NULL, updated_at = now()
        WHERE request_id = $2
          AND node_id = $3
          AND round = $4
          AND status IN ($5, $6)
        RETURNING node_id, round, status, public_payload
        "#,
    )
    .bind(STATUS_RUNNING)
    .bind(request_id)
    .bind(node_id)
    .bind(round)
    .bind(STATUS_NOT_STARTED)
    .bind(STATUS_FAILED)
    .fetch_optional(pool)
    .await
    .map_err(DkgError::from)
}

async fn run_sql_file(pool: &PgPool, sql: &str) -> Result<(), sqlx::Error> {
    for statement in sql
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        pool.execute(statement).await?;
    }

    Ok(())
}

async fn fetch_signing_request(
    pool: &PgPool,
    request_id: Uuid,
) -> Result<Option<SigningRequestRow>, DkgError> {
    sqlx::query_as::<_, SigningRequestRow>(
        r#"
        SELECT
            id,
            wallet_index,
            sender_address_base58,
            recipient_address_base58,
            amount_lamports,
            status,
            message_payload,
            message_hash_hex,
            recent_blockhash,
            transaction_signature,
            explorer_url,
            error_message,
            created_at::text AS created_at,
            updated_at::text AS updated_at
        FROM coordinator.signing_requests
        WHERE id = $1
        "#,
    )
    .bind(request_id)
    .fetch_optional(pool)
    .await
    .map_err(DkgError::from)
}

async fn fetch_signing_requests(
    pool: &PgPool,
    status_filter: Option<&str>,
) -> Result<Vec<SigningRequestRow>, DkgError> {
    let normalized_status = status_filter.map(|status| status.trim().to_ascii_uppercase());

    if normalized_status.as_deref() == Some("PENDING") {
        return sqlx::query_as::<_, SigningRequestRow>(
            r#"
            SELECT
                id,
                wallet_index,
                sender_address_base58,
                recipient_address_base58,
                amount_lamports,
                status,
                message_payload,
                message_hash_hex,
                recent_blockhash,
                transaction_signature,
                explorer_url,
                error_message,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM coordinator.signing_requests
            WHERE status IN ($1, $2, $3, $4, $5)
            ORDER BY created_at DESC
            "#,
        )
        .bind(SIGNING_STATUS_PENDING)
        .bind(SIGNING_STATUS_COMMITMENTS_IN_PROGRESS)
        .bind(SIGNING_STATUS_COMMITMENTS_READY)
        .bind(SIGNING_STATUS_SHARES_IN_PROGRESS)
        .bind(SIGNING_STATUS_READY_TO_AGGREGATE)
        .fetch_all(pool)
        .await
        .map_err(DkgError::from);
    }

    if let Some(status) = normalized_status {
        return sqlx::query_as::<_, SigningRequestRow>(
            r#"
            SELECT
                id,
                wallet_index,
                sender_address_base58,
                recipient_address_base58,
                amount_lamports,
                status,
                message_payload,
                message_hash_hex,
                recent_blockhash,
                transaction_signature,
                explorer_url,
                error_message,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM coordinator.signing_requests
            WHERE status = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(status)
        .fetch_all(pool)
        .await
        .map_err(DkgError::from);
    }

    sqlx::query_as::<_, SigningRequestRow>(
        r#"
        SELECT
            id,
            wallet_index,
            sender_address_base58,
            recipient_address_base58,
            amount_lamports,
            status,
            message_payload,
            message_hash_hex,
            recent_blockhash,
            transaction_signature,
            explorer_url,
            error_message,
            created_at::text AS created_at,
            updated_at::text AS updated_at
        FROM coordinator.signing_requests
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(DkgError::from)
}

async fn fetch_signing_steps(
    pool: &PgPool,
    request_id: Uuid,
) -> Result<Vec<SigningStepRow>, DkgError> {
    sqlx::query_as::<_, SigningStepRow>(
        r#"
        SELECT node_id, round, status, public_payload
        FROM coordinator.signing_node_steps
        WHERE request_id = $1
        ORDER BY node_id, round
        "#,
    )
    .bind(request_id)
    .fetch_all(pool)
    .await
    .map_err(DkgError::from)
}

async fn fetch_signing_step(
    pool: &PgPool,
    request_id: Uuid,
    node_id: &str,
    round: i32,
) -> Result<Option<SigningStepRow>, DkgError> {
    sqlx::query_as::<_, SigningStepRow>(
        r#"
        SELECT node_id, round, status, public_payload
        FROM coordinator.signing_node_steps
        WHERE request_id = $1 AND node_id = $2 AND round = $3
        "#,
    )
    .bind(request_id)
    .bind(node_id)
    .bind(round)
    .fetch_optional(pool)
    .await
    .map_err(DkgError::from)
}

async fn fetch_signing_request_response(
    pool: &PgPool,
    request: SigningRequestRow,
) -> Result<SigningRequestResponse, DkgError> {
    let steps = fetch_signing_steps(pool, request.id).await?;

    Ok(signing_request_response_from_rows(request, steps))
}

async fn completed_wallet_session(pool: &PgPool) -> Result<DkgSessionRow, DkgError> {
    let session = fetch_active_session(pool)
        .await?
        .ok_or_else(wallet_derivation_prerequisite_error)?;

    validate_completed_wallet_session(&session)?;

    Ok(session)
}

fn validate_completed_wallet_session(session: &DkgSessionRow) -> Result<(), DkgError> {
    if session.status != STATUS_COMPLETED || session.master_public_key_base58.is_none() {
        return Err(wallet_derivation_prerequisite_error());
    }

    Ok(())
}

fn wallet_derivation_prerequisite_error() -> DkgError {
    DkgError::WalletDerivationBlocked(
        "wallet derivation requires a completed DKG session with a master public key".to_string(),
    )
}

async fn ensure_public_derivation_context(
    pool: &PgPool,
    session: &DkgSessionRow,
) -> Result<Value, DkgError> {
    if let Some(context) = &session.public_derivation_context {
        validate_public_derivation_context(&context.0)?;
        return Ok(context.0.clone());
    }

    let master_public_key = session
        .master_public_key_base58
        .as_deref()
        .ok_or_else(wallet_derivation_prerequisite_error)?;
    let context = build_public_derivation_context(master_public_key);

    sqlx::query(
        r#"
        UPDATE coordinator.dkg_sessions
        SET public_derivation_context = COALESCE(public_derivation_context, $1),
            updated_at = now()
        WHERE id = $2
        "#,
    )
    .bind(SqlxJson(context.clone()))
    .bind(session.id)
    .execute(pool)
    .await?;

    Ok(context)
}

fn build_public_derivation_context(master_public_key_base58: &str) -> Value {
    let mut hasher = Sha256::new();
    hasher.update(PUBLIC_DERIVATION_CONTEXT_DOMAIN.as_bytes());
    hasher.update(master_public_key_base58.as_bytes());
    let chain_code = hasher.finalize();

    json!({
        "scheme": PUBLIC_DERIVATION_SCHEME,
        "chain_code_base58": bs58::encode(chain_code.as_slice()).into_string()
    })
}

fn validate_public_derivation_context(context: &Value) -> Result<(), DkgError> {
    if context.get("scheme").and_then(Value::as_str) != Some(PUBLIC_DERIVATION_SCHEME) {
        return Err(DkgError::WalletDerivationFailed(
            "public derivation context has an unsupported scheme".to_string(),
        ));
    }

    decode_public_derivation_chain_code(context).map(|_| ())
}

fn derive_wallet(
    master_public_key_base58: &str,
    public_derivation_context: &Value,
    wallet_index: i32,
) -> Result<DerivedWallet, DkgError> {
    validate_wallet_index(wallet_index)?;
    let master_public_key_bytes =
        bs58::decode(master_public_key_base58)
            .into_vec()
            .map_err(|error| {
                DkgError::WalletDerivationFailed(format!(
                    "master public key is not valid Base58: {error}"
                ))
            })?;
    let parent_public_key =
        Point::<Ed25519>::from_bytes(&master_public_key_bytes).map_err(|_| {
            DkgError::WalletDerivationFailed(
                "master public key is not a valid Ed25519 point".to_string(),
            )
        })?;
    let chain_code = decode_public_derivation_chain_code(public_derivation_context)?;
    let extended_public_key = ExtendedPublicKey::<Ed25519> {
        public_key: parent_public_key,
        chain_code,
    };
    let child_index = NonHardenedIndex::try_from(wallet_index as u32)
        .map_err(|_| DkgError::InvalidWalletIndex(wallet_index))?;
    let child_public_key =
        edwards::derive_child_public_key_with_path(&extended_public_key, [child_index]);
    let child_public_key_bytes = child_public_key.public_key.to_bytes(true);
    let public_key_base58 = bs58::encode(child_public_key_bytes.as_bytes()).into_string();

    Ok(DerivedWallet {
        wallet_index,
        derivation_path: format!("m/{wallet_index}"),
        public_key_base58: public_key_base58.clone(),
        address_base58: public_key_base58,
    })
}

fn decode_public_derivation_chain_code(context: &Value) -> Result<[u8; 32], DkgError> {
    let chain_code_base58 = context
        .get("chain_code_base58")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            DkgError::WalletDerivationFailed(
                "public derivation context is missing chain_code_base58".to_string(),
            )
        })?;
    let chain_code_bytes = bs58::decode(chain_code_base58)
        .into_vec()
        .map_err(|error| {
            DkgError::WalletDerivationFailed(format!(
                "public derivation chain code is not valid Base58: {error}"
            ))
        })?;

    chain_code_bytes.try_into().map_err(|_| {
        DkgError::WalletDerivationFailed(
            "public derivation chain code must be 32 bytes".to_string(),
        )
    })
}

fn validate_wallet_index(wallet_index: i32) -> Result<(), DkgError> {
    if wallet_index < 0 {
        return Err(DkgError::InvalidWalletIndex(wallet_index));
    }

    Ok(())
}

async fn fetch_wallets(pool: &PgPool) -> Result<Vec<WalletRow>, DkgError> {
    sqlx::query_as::<_, WalletRow>(
        r#"
        SELECT
            wallet_index,
            dkg_session_id,
            derivation_path,
            public_key_base58,
            address_base58,
            balance_lamports,
            balance_status,
            balance_error_message,
            balance_checked_at::text AS balance_checked_at,
            created_at::text AS created_at
        FROM coordinator.wallets
        ORDER BY wallet_index
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(DkgError::from)
}

async fn fetch_wallet(pool: &PgPool, wallet_index: i32) -> Result<Option<WalletRow>, DkgError> {
    sqlx::query_as::<_, WalletRow>(
        r#"
        SELECT
            wallet_index,
            dkg_session_id,
            derivation_path,
            public_key_base58,
            address_base58,
            balance_lamports,
            balance_status,
            balance_error_message,
            balance_checked_at::text AS balance_checked_at,
            created_at::text AS created_at
        FROM coordinator.wallets
        WHERE wallet_index = $1
        "#,
    )
    .bind(wallet_index)
    .fetch_optional(pool)
    .await
    .map_err(DkgError::from)
}

async fn update_wallet_balance(
    pool: &PgPool,
    wallet_index: i32,
    balance_lamports: Option<i64>,
    balance_status: &str,
    balance_error_message: Option<&str>,
) -> Result<WalletRow, DkgError> {
    sqlx::query_as::<_, WalletRow>(
        r#"
        UPDATE coordinator.wallets
        SET balance_lamports = $2,
            balance_status = $3,
            balance_error_message = $4,
            balance_checked_at = now()
        WHERE wallet_index = $1
        RETURNING
            wallet_index,
            dkg_session_id,
            derivation_path,
            public_key_base58,
            address_base58,
            balance_lamports,
            balance_status,
            balance_error_message,
            balance_checked_at::text AS balance_checked_at,
            created_at::text AS created_at
        "#,
    )
    .bind(wallet_index)
    .bind(balance_lamports)
    .bind(balance_status)
    .bind(balance_error_message)
    .fetch_optional(pool)
    .await?
    .ok_or(DkgError::WalletNotFound(wallet_index))
}

async fn fetch_balance_lamports(
    client: &Client,
    solana_rpc_url: &str,
    address_base58: &str,
) -> Result<i64, String> {
    let response = client
        .post(solana_rpc_url)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [address_base58]
        }))
        .send()
        .await
        .map_err(|_| "Solana RPC request failed".to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(format!("Solana RPC returned HTTP {status}"));
    }

    let payload = response
        .json::<SolanaBalanceRpcResponse>()
        .await
        .map_err(|_| "Solana RPC response was invalid".to_string())?;

    if let Some(error) = payload.error {
        return Err(format!(
            "Solana RPC error: {}",
            public_error_message(&error.message)
        ));
    }

    let balance = payload
        .result
        .ok_or_else(|| "Solana RPC response did not include a balance".to_string())?
        .value;

    i64::try_from(balance).map_err(|_| "Solana RPC balance exceeded i64 range".to_string())
}

fn public_error_message(message: &str) -> String {
    const MAX_PUBLIC_ERROR_CHARS: usize = 160;
    let compact = message.split_whitespace().collect::<Vec<_>>().join(" ");

    if compact.chars().count() <= MAX_PUBLIC_ERROR_CHARS {
        return compact;
    }

    format!(
        "{}...",
        compact
            .chars()
            .take(MAX_PUBLIC_ERROR_CHARS)
            .collect::<String>()
    )
}

impl From<WalletRow> for WalletResponse {
    fn from(row: WalletRow) -> Self {
        Self {
            wallet_index: row.wallet_index,
            dkg_session_id: row.dkg_session_id,
            derivation_path: row.derivation_path,
            public_key_base58: row.public_key_base58,
            address_base58: row.address_base58,
            balance_lamports: row.balance_lamports,
            balance_status: row.balance_status,
            balance_error_message: row.balance_error_message,
            balance_checked_at: row.balance_checked_at,
            created_at: row.created_at,
        }
    }
}

fn signing_request_response_from_rows(
    row: SigningRequestRow,
    steps: Vec<SigningStepRow>,
) -> SigningRequestResponse {
    SigningRequestResponse {
        request_id: row.id,
        wallet_index: row.wallet_index,
        sender_address_base58: row.sender_address_base58,
        recipient_address_base58: row.recipient_address_base58,
        amount_lamports: row.amount_lamports,
        status: row.status,
        message_hash_hex: row.message_hash_hex,
        recent_blockhash: row.recent_blockhash,
        transaction_signature: row.transaction_signature,
        explorer_url: row.explorer_url,
        error_message: row.error_message,
        created_at: row.created_at,
        updated_at: row.updated_at,
        node_steps: steps
            .into_iter()
            .map(|step| SigningNodeStepResponse {
                node_id: step.node_id,
                round: step.round,
                status: step.status,
            })
            .collect(),
    }
}

fn validate_create_request(payload: &CreateDkgSessionRequest) -> Result<(), DkgError> {
    if payload.threshold != 2 {
        return Err(DkgError::InvalidCreateRequest(
            "only threshold 2 is supported in this demo".to_string(),
        ));
    }

    let mut participants = payload.participants.clone();
    participants.sort();

    if participants != NODE_IDS {
        return Err(DkgError::InvalidCreateRequest(
            "participants must be exactly node-a and node-b".to_string(),
        ));
    }

    Ok(())
}

fn validate_create_signing_request(
    payload: &CreateSigningRequestRequest,
) -> Result<(), DkgError> {
    validate_wallet_index(payload.wallet_index)?;

    if payload.amount_lamports <= 0 {
        return Err(DkgError::InvalidSigningRequest(
            "amount_lamports must be positive".to_string(),
        ));
    }

    let recipient_bytes = bs58::decode(&payload.recipient_address_base58)
        .into_vec()
        .map_err(|error| {
            DkgError::InvalidSigningRequest(format!(
                "recipient_address_base58 must be valid Base58: {error}"
            ))
        })?;

    if recipient_bytes.len() != 32 {
        return Err(DkgError::InvalidSigningRequest(
            "recipient_address_base58 must decode to 32 bytes".to_string(),
        ));
    }

    Ok(())
}

fn validate_node_id(node_id: &str) -> Result<(), DkgError> {
    if NODE_IDS.contains(&node_id) {
        Ok(())
    } else {
        Err(DkgError::InvalidNode(node_id.to_string()))
    }
}

fn validate_round(round: i32) -> Result<(), DkgError> {
    if DKG_ROUNDS.contains(&round) {
        Ok(())
    } else {
        Err(DkgError::InvalidRound(round))
    }
}

fn validate_signing_round(round: i32) -> Result<(), DkgError> {
    if SIGNING_ROUNDS.contains(&round) {
        Ok(())
    } else {
        Err(DkgError::InvalidSigningRound(round))
    }
}

fn validate_round_prerequisites(steps: &[DkgStepRow], round: i32) -> Result<(), DkgError> {
    match round {
        1 => Ok(()),
        2 if all_round_steps_completed(steps, 1) => Ok(()),
        2 => Err(DkgError::TransitionBlocked(
            "round 2 requires both round 1 steps to be completed".to_string(),
        )),
        3 if all_round_steps_completed(steps, 2) => Ok(()),
        3 => Err(DkgError::TransitionBlocked(
            "round 3 requires both round 2 steps to be completed".to_string(),
        )),
        _ => Err(DkgError::InvalidRound(round)),
    }
}

fn validate_signing_round_prerequisites(
    steps: &[SigningStepRow],
    round: i32,
) -> Result<(), DkgError> {
    match round {
        1 => Ok(()),
        2 if all_signing_round_steps_completed(steps, 1) => Ok(()),
        2 => Err(DkgError::SigningTransitionBlocked(
            "signing round 2 requires both round 1 commitments to be completed".to_string(),
        )),
        _ => Err(DkgError::InvalidSigningRound(round)),
    }
}

fn build_node_dkg_round_request(
    steps: &[DkgStepRow],
    node_id: &str,
    round: i32,
) -> Result<NodeDkgRoundRequest, DkgError> {
    match round {
        1 => Ok(NodeDkgRoundRequest::default()),
        2 => Ok(NodeDkgRoundRequest {
            peer_round1_packages: peer_round1_packages(steps, node_id)?,
            peer_round2_packages: BTreeMap::new(),
        }),
        3 => Ok(NodeDkgRoundRequest {
            peer_round1_packages: peer_round1_packages(steps, node_id)?,
            peer_round2_packages: peer_round2_packages(steps, node_id)?,
        }),
        _ => Err(DkgError::InvalidRound(round)),
    }
}

async fn build_node_signing_round_request(
    pool: &PgPool,
    request: &SigningRequestRow,
    steps: &[SigningStepRow],
    _node_id: &str,
    round: i32,
) -> Result<NodeSigningRoundRequest, DkgError> {
    let (message_payload, message_hash_hex) = if round == 2 {
        ensure_signing_message(pool, request).await?
    } else {
        (Value::Null, String::new())
    };

    Ok(NodeSigningRoundRequest {
        wallet_index: request.wallet_index,
        sender_address_base58: request.sender_address_base58.clone(),
        recipient_address_base58: request.recipient_address_base58.clone(),
        amount_lamports: request.amount_lamports,
        message_payload,
        message_hash_hex,
        signing_commitments: if round == 2 {
            signing_commitments(steps)?
        } else {
            BTreeMap::new()
        },
    })
}

async fn ensure_signing_message(
    pool: &PgPool,
    request: &SigningRequestRow,
) -> Result<(Value, String), DkgError> {
    if let (Some(payload), Some(message_hash_hex)) =
        (&request.message_payload, &request.message_hash_hex)
    {
        return Ok((payload.0.clone(), message_hash_hex.clone()));
    }

    let canonical_message = canonical_transfer_message(request);
    let mut hasher = Sha256::new();
    hasher.update(canonical_message.as_bytes());
    let message_hash_hex = hex_string(&hasher.finalize());
    let payload = json!({
        "format": SIGNING_MESSAGE_FORMAT,
        "request_id": request.id,
        "wallet_index": request.wallet_index,
        "sender_address_base58": request.sender_address_base58,
        "recipient_address_base58": request.recipient_address_base58,
        "amount_lamports": request.amount_lamports,
        "canonical_message": canonical_message
    });

    sqlx::query(
        r#"
        UPDATE coordinator.signing_requests
        SET message_payload = COALESCE(message_payload, $1),
            message_hash_hex = COALESCE(message_hash_hex, $2),
            updated_at = now()
        WHERE id = $3
        "#,
    )
    .bind(SqlxJson(payload.clone()))
    .bind(&message_hash_hex)
    .bind(request.id)
    .execute(pool)
    .await?;

    Ok((payload, message_hash_hex))
}

fn canonical_transfer_message(request: &SigningRequestRow) -> String {
    [
        SIGNING_MESSAGE_FORMAT.to_string(),
        format!("request_id={}", request.id),
        format!("wallet_index={}", request.wallet_index),
        format!("sender_address_base58={}", request.sender_address_base58),
        format!(
            "recipient_address_base58={}",
            request.recipient_address_base58
        ),
        format!("amount_lamports={}", request.amount_lamports),
    ]
    .join("\n")
}

fn signing_commitments(steps: &[SigningStepRow]) -> Result<BTreeMap<String, String>, DkgError> {
    let mut commitments = BTreeMap::new();

    for step in steps.iter().filter(|step| step.round == 1) {
        commitments.insert(
            step.node_id.clone(),
            signing_required_payload_string(step, "commitments_hex")?,
        );
    }

    if commitments.len() != NODE_IDS.len() {
        return Err(DkgError::SigningTransitionBlocked(
            "signing commitments are not ready".to_string(),
        ));
    }

    Ok(commitments)
}

fn peer_round1_packages(
    steps: &[DkgStepRow],
    current_node_id: &str,
) -> Result<BTreeMap<String, String>, DkgError> {
    let mut packages = BTreeMap::new();

    for step in steps
        .iter()
        .filter(|step| step.round == 1 && step.node_id != current_node_id)
    {
        packages.insert(
            step.node_id.clone(),
            required_payload_string(step, "public_package_hex")?,
        );
    }

    if packages.len() != NODE_IDS.len() - 1 {
        return Err(DkgError::TransitionBlocked(
            "peer round 1 packages are not ready".to_string(),
        ));
    }

    Ok(packages)
}

fn peer_round2_packages(
    steps: &[DkgStepRow],
    current_node_id: &str,
) -> Result<BTreeMap<String, String>, DkgError> {
    let mut packages = BTreeMap::new();

    for step in steps
        .iter()
        .filter(|step| step.round == 2 && step.node_id != current_node_id)
    {
        let payload = step_payload(step)?;
        let package = payload
            .get("round2_packages")
            .and_then(|packages| packages.get(current_node_id))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                DkgError::TransitionBlocked(format!(
                    "{} round 2 payload does not include a package for {current_node_id}",
                    step.node_id
                ))
            })?;
        packages.insert(step.node_id.clone(), package.to_string());
    }

    if packages.len() != NODE_IDS.len() - 1 {
        return Err(DkgError::TransitionBlocked(
            "peer round 2 packages are not ready".to_string(),
        ));
    }

    Ok(packages)
}

fn extract_completed_master_public_key(steps: &[DkgStepRow]) -> Result<String, DkgError> {
    let mut master_public_keys = Vec::new();

    for step in steps.iter().filter(|step| step.round == 3) {
        master_public_keys.push(required_payload_string(step, "master_public_key_base58")?);
    }

    let first = master_public_keys.first().ok_or_else(|| {
        DkgError::NodeCallFailed("completed DKG session has no master public key".to_string())
    })?;

    if master_public_keys.iter().any(|key| key != first) {
        return Err(DkgError::NodeCallFailed(
            "TSS nodes returned different master public keys".to_string(),
        ));
    }

    Ok(first.clone())
}

fn required_payload_string(step: &DkgStepRow, field: &str) -> Result<String, DkgError> {
    step_payload(step)?
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            DkgError::TransitionBlocked(format!(
                "{} round {} payload is missing {field}",
                step.node_id, step.round
            ))
        })
}

fn step_payload(step: &DkgStepRow) -> Result<&Value, DkgError> {
    step.public_payload
        .as_ref()
        .map(|payload| &payload.0)
        .ok_or_else(|| {
            DkgError::TransitionBlocked(format!(
                "{} round {} has no stored public payload",
                step.node_id, step.round
            ))
        })
}

fn signing_required_payload_string(
    step: &SigningStepRow,
    field: &str,
) -> Result<String, DkgError> {
    signing_step_payload(step)?
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            DkgError::SigningTransitionBlocked(format!(
                "{} signing round {} payload is missing {field}",
                step.node_id, step.round
            ))
        })
}

fn signing_step_payload(step: &SigningStepRow) -> Result<&Value, DkgError> {
    step.public_payload
        .as_ref()
        .map(|payload| &payload.0)
        .ok_or_else(|| {
            DkgError::SigningTransitionBlocked(format!(
                "{} signing round {} has no stored public payload",
                step.node_id, step.round
            ))
        })
}

fn derive_dkg_status(steps: &[DkgStepRow]) -> String {
    if all_round_steps_completed(steps, 3) {
        return STATUS_COMPLETED.to_string();
    }
    if any_round_step_started(steps, 3) {
        return STATUS_ROUND_3_IN_PROGRESS.to_string();
    }
    if all_round_steps_completed(steps, 2) {
        return STATUS_ROUND_2_COMPLETE.to_string();
    }
    if any_round_step_started(steps, 2) {
        return STATUS_ROUND_2_IN_PROGRESS.to_string();
    }
    if all_round_steps_completed(steps, 1) {
        return STATUS_ROUND_1_COMPLETE.to_string();
    }
    if any_round_step_started(steps, 1) {
        return STATUS_ROUND_1_IN_PROGRESS.to_string();
    }

    STATUS_NOT_STARTED.to_string()
}

fn derive_signing_status(steps: &[SigningStepRow]) -> String {
    if all_signing_round_steps_completed(steps, 2) {
        return SIGNING_STATUS_READY_TO_AGGREGATE.to_string();
    }
    if any_signing_round_step_started(steps, 2) {
        return SIGNING_STATUS_SHARES_IN_PROGRESS.to_string();
    }
    if all_signing_round_steps_completed(steps, 1) {
        return SIGNING_STATUS_COMMITMENTS_READY.to_string();
    }
    if any_signing_round_step_started(steps, 1) {
        return SIGNING_STATUS_COMMITMENTS_IN_PROGRESS.to_string();
    }

    SIGNING_STATUS_PENDING.to_string()
}

fn all_round_steps_completed(steps: &[DkgStepRow], round: i32) -> bool {
    let round_steps: Vec<&DkgStepRow> = steps.iter().filter(|step| step.round == round).collect();

    round_steps.len() == NODE_IDS.len()
        && round_steps
            .iter()
            .all(|step| step.status == STATUS_COMPLETED)
}

fn all_signing_round_steps_completed(steps: &[SigningStepRow], round: i32) -> bool {
    let round_steps: Vec<&SigningStepRow> =
        steps.iter().filter(|step| step.round == round).collect();

    round_steps.len() == NODE_IDS.len()
        && round_steps
            .iter()
            .all(|step| step.status == STATUS_COMPLETED)
}

fn any_round_step_started(steps: &[DkgStepRow], round: i32) -> bool {
    steps
        .iter()
        .any(|step| step.round == round && step.status != STATUS_NOT_STARTED)
}

fn any_signing_round_step_started(steps: &[SigningStepRow], round: i32) -> bool {
    steps
        .iter()
        .any(|step| step.round == round && step.status != STATUS_NOT_STARTED)
}

async fn fetch_active_session(pool: &PgPool) -> Result<Option<DkgSessionRow>, DkgError> {
    sqlx::query_as::<_, DkgSessionRow>(
        r#"
        SELECT id, status, master_public_key_base58, public_derivation_context
        FROM coordinator.dkg_sessions
        WHERE active = TRUE
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(DkgError::from)
}

async fn fetch_session(pool: &PgPool, session_id: Uuid) -> Result<Option<DkgSessionRow>, DkgError> {
    sqlx::query_as::<_, DkgSessionRow>(
        r#"
        SELECT id, status, master_public_key_base58, public_derivation_context
        FROM coordinator.dkg_sessions
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .map_err(DkgError::from)
}

async fn fetch_session_steps(pool: &PgPool, session_id: Uuid) -> Result<Vec<DkgStepRow>, DkgError> {
    sqlx::query_as::<_, DkgStepRow>(
        r#"
        SELECT node_id, round, status, public_payload
        FROM coordinator.dkg_node_steps
        WHERE session_id = $1
        ORDER BY node_id, round
        "#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .map_err(DkgError::from)
}

async fn fetch_session_step(
    pool: &PgPool,
    session_id: Uuid,
    node_id: &str,
    round: i32,
) -> Result<Option<DkgStepRow>, DkgError> {
    sqlx::query_as::<_, DkgStepRow>(
        r#"
        SELECT node_id, round, status, public_payload
        FROM coordinator.dkg_node_steps
        WHERE session_id = $1 AND node_id = $2 AND round = $3
        "#,
    )
    .bind(session_id)
    .bind(node_id)
    .bind(round)
    .fetch_optional(pool)
    .await
    .map_err(DkgError::from)
}

async fn fetch_session_response(
    pool: &PgPool,
    session: DkgSessionRow,
) -> Result<DkgSessionResponse, DkgError> {
    let steps = fetch_session_steps(pool, session.id).await?;

    Ok(session_response_from_rows(session, steps))
}

fn session_response_from_rows(
    session: DkgSessionRow,
    steps: Vec<DkgStepRow>,
) -> DkgSessionResponse {
    DkgSessionResponse {
        session_id: session.id,
        status: session.status,
        master_public_key_base58: session.master_public_key_base58,
        node_steps: steps
            .into_iter()
            .map(|step| DkgNodeStepResponse {
                node_id: step.node_id,
                round: step.round,
                status: step.status,
            })
            .collect(),
    }
}

fn completed_step_response(session: &DkgSessionRow, step: &DkgStepRow) -> TriggerDkgRoundResponse {
    TriggerDkgRoundResponse {
        session_id: session.id,
        node_id: step.node_id.clone(),
        round: step.round,
        status: STATUS_COMPLETED.to_string(),
        dkg_status: session.status.clone(),
        public_payload: step
            .public_payload
            .as_ref()
            .map(|payload| client_public_payload(&payload.0)),
    }
}

fn completed_signing_step_response(
    request: &SigningRequestRow,
    step: &SigningStepRow,
) -> TriggerSigningRoundResponse {
    TriggerSigningRoundResponse {
        request_id: request.id,
        node_id: step.node_id.clone(),
        round: step.round,
        status: STATUS_COMPLETED.to_string(),
        signing_status: request.status.clone(),
        public_payload: step.public_payload.as_ref().map(|payload| payload.0.clone()),
    }
}

fn client_public_payload(payload: &Value) -> Value {
    if payload.get("kind").and_then(Value::as_str) != Some("frost-dkg-round2") {
        return payload.clone();
    }

    let mut redacted = payload.clone();

    if let Some(object) = redacted.as_object_mut() {
        object.remove("round2_packages");
        object.insert(
            "routing_payload".to_string(),
            Value::String("stored-in-coordinator".to_string()),
        );
    }

    redacted
}

fn hex_string(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }

    output
}

fn db_pool(state: &AppState) -> Result<&PgPool, DkgError> {
    state.db_pool.as_ref().ok_or(DkgError::DatabaseUnavailable)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .is_some_and(|code| code.as_ref() == "23505")
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

    #[test]
    fn blocks_round_two_until_both_round_one_steps_complete() {
        let steps = vec![
            step("node-a", 1, STATUS_COMPLETED),
            step("node-b", 1, STATUS_NOT_STARTED),
            step("node-a", 2, STATUS_NOT_STARTED),
            step("node-b", 2, STATUS_NOT_STARTED),
        ];

        let error = validate_round_prerequisites(&steps, 2)
            .expect_err("round 2 should wait for both round 1 steps");

        assert!(matches!(error, DkgError::TransitionBlocked(_)));
    }

    #[test]
    fn blocks_round_three_until_both_round_two_steps_complete() {
        let steps = vec![
            step("node-a", 1, STATUS_COMPLETED),
            step("node-b", 1, STATUS_COMPLETED),
            step("node-a", 2, STATUS_COMPLETED),
            step("node-b", 2, STATUS_NOT_STARTED),
            step("node-a", 3, STATUS_NOT_STARTED),
            step("node-b", 3, STATUS_NOT_STARTED),
        ];

        let error = validate_round_prerequisites(&steps, 3)
            .expect_err("round 3 should wait for both round 2 steps");

        assert!(matches!(error, DkgError::TransitionBlocked(_)));
    }

    #[test]
    fn derives_completed_session_after_both_round_three_steps_complete() {
        let steps = all_completed_steps();

        assert_eq!(derive_dkg_status(&steps), STATUS_COMPLETED);
    }

    #[test]
    fn blocks_signing_round_two_until_both_commitments_complete() {
        let steps = vec![
            signing_step("node-a", 1, STATUS_COMPLETED),
            signing_step("node-b", 1, STATUS_NOT_STARTED),
            signing_step("node-a", 2, STATUS_NOT_STARTED),
            signing_step("node-b", 2, STATUS_NOT_STARTED),
        ];

        let error = validate_signing_round_prerequisites(&steps, 2)
            .expect_err("round 2 should wait for both signing commitments");

        assert!(matches!(error, DkgError::SigningTransitionBlocked(_)));
    }

    #[test]
    fn derives_ready_to_aggregate_after_both_signature_shares_complete() {
        let steps = vec![
            signing_step("node-a", 1, STATUS_COMPLETED),
            signing_step("node-b", 1, STATUS_COMPLETED),
            signing_step("node-a", 2, STATUS_COMPLETED),
            signing_step("node-b", 2, STATUS_COMPLETED),
        ];

        assert_eq!(
            derive_signing_status(&steps),
            SIGNING_STATUS_READY_TO_AGGREGATE
        );
    }

    #[test]
    fn signing_commitments_require_both_node_payloads() {
        let steps = vec![
            signing_step_with_payload(
                "node-a",
                1,
                json!({ "commitments_hex": "node-a-commitment" }),
            ),
            signing_step_with_payload(
                "node-b",
                1,
                json!({ "commitments_hex": "node-b-commitment" }),
            ),
        ];

        let commitments = signing_commitments(&steps).expect("commitments should collect");

        assert_eq!(
            commitments,
            BTreeMap::from([
                ("node-a".to_string(), "node-a-commitment".to_string()),
                ("node-b".to_string(), "node-b-commitment".to_string())
            ])
        );
    }

    #[test]
    fn validates_signing_transfer_inputs() {
        let request = CreateSigningRequestRequest {
            wallet_index: 0,
            recipient_address_base58: bs58::encode([1_u8; 32]).into_string(),
            amount_lamports: 1,
        };

        validate_create_signing_request(&request).expect("request should be valid");

        let invalid_amount = CreateSigningRequestRequest {
            amount_lamports: 0,
            ..request.clone()
        };
        let error = validate_create_signing_request(&invalid_amount)
            .expect_err("zero amount should fail");

        assert!(matches!(error, DkgError::InvalidSigningRequest(_)));
    }

    #[test]
    fn retriggering_completed_step_returns_stored_public_payload() {
        let session_id = Uuid::new_v4();
        let session = DkgSessionRow {
            id: session_id,
            status: STATUS_ROUND_1_IN_PROGRESS.to_string(),
            master_public_key_base58: None,
            public_derivation_context: None,
        };
        let completed_step = DkgStepRow {
            node_id: "node-a".to_string(),
            round: 1,
            status: STATUS_COMPLETED.to_string(),
            public_payload: Some(SqlxJson(json!({
                "kind": "stored-round-result",
                "node_id": "node-a",
                "round": 1
            }))),
        };

        let response = completed_step_response(&session, &completed_step);

        assert_eq!(response.session_id, session_id);
        assert_eq!(response.status, STATUS_COMPLETED);
        assert_eq!(response.dkg_status, STATUS_ROUND_1_IN_PROGRESS);
        assert_eq!(
            response.public_payload,
            Some(json!({
                "kind": "stored-round-result",
                "node_id": "node-a",
                "round": 1
            }))
        );
    }

    #[test]
    fn completed_round_two_response_redacts_routing_packages_for_clients() {
        let session_id = Uuid::new_v4();
        let session = DkgSessionRow {
            id: session_id,
            status: STATUS_ROUND_2_IN_PROGRESS.to_string(),
            master_public_key_base58: None,
            public_derivation_context: None,
        };
        let completed_step = DkgStepRow {
            node_id: "node-a".to_string(),
            round: 2,
            status: STATUS_COMPLETED.to_string(),
            public_payload: Some(SqlxJson(json!({
                "kind": "frost-dkg-round2",
                "node_id": "node-a",
                "round": 2,
                "round2_packages": {
                    "node-b": "recipient-specific-package"
                }
            }))),
        };

        let response = completed_step_response(&session, &completed_step);
        let payload = response
            .public_payload
            .expect("completed step should include public payload");

        assert_eq!(payload["kind"], "frost-dkg-round2");
        assert_eq!(payload["routing_payload"], "stored-in-coordinator");
        assert!(payload.get("round2_packages").is_none());
    }

    #[test]
    fn completed_session_response_keeps_reloaded_master_public_key() {
        let session_id = Uuid::new_v4();
        let master_public_key = "7Y9mEJ8h7A4n9Jx9uG2Xo4BfPuTQ3bYQyCMwFQFrost".to_string();
        let session = DkgSessionRow {
            id: session_id,
            status: STATUS_COMPLETED.to_string(),
            master_public_key_base58: Some(master_public_key.clone()),
            public_derivation_context: None,
        };

        let response = session_response_from_rows(session, all_completed_steps());

        assert_eq!(response.session_id, session_id);
        assert_eq!(response.status, STATUS_COMPLETED);
        assert_eq!(response.master_public_key_base58, Some(master_public_key));
        assert_eq!(response.node_steps.len(), 6);
    }

    #[test]
    fn rejects_wallet_derivation_before_dkg_completes() {
        let session = DkgSessionRow {
            id: Uuid::new_v4(),
            status: STATUS_ROUND_2_COMPLETE.to_string(),
            master_public_key_base58: Some(sample_master_public_key_base58()),
            public_derivation_context: None,
        };

        let error = validate_completed_wallet_session(&session)
            .expect_err("wallet derivation should wait for completed DKG");

        assert!(matches!(error, DkgError::WalletDerivationBlocked(_)));
    }

    #[test]
    fn public_derivation_context_is_deterministic() {
        let master_public_key = sample_master_public_key_base58();
        let first = build_public_derivation_context(&master_public_key);
        let second = build_public_derivation_context(&master_public_key);

        assert_eq!(first, second);
        assert_eq!(first["scheme"], PUBLIC_DERIVATION_SCHEME);
        assert_eq!(
            decode_public_derivation_chain_code(&first)
                .expect("chain code should decode")
                .len(),
            32
        );
    }

    #[test]
    fn derives_wallet_deterministically_for_same_context_and_index() {
        let master_public_key = sample_master_public_key_base58();
        let context = build_public_derivation_context(&master_public_key);
        let first = derive_wallet(&master_public_key, &context, 0)
            .expect("first derivation should succeed");
        let second = derive_wallet(&master_public_key, &context, 0)
            .expect("second derivation should succeed");

        assert_eq!(first, second);
        assert_eq!(first.wallet_index, 0);
        assert_eq!(first.derivation_path, "m/0");
        assert_eq!(first.public_key_base58, first.address_base58);
        assert!(!first.address_base58.is_empty());
    }

    #[test]
    fn derives_different_wallets_for_different_indexes() {
        let master_public_key = sample_master_public_key_base58();
        let context = build_public_derivation_context(&master_public_key);
        let first = derive_wallet(&master_public_key, &context, 0).expect("index 0 should derive");
        let second = derive_wallet(&master_public_key, &context, 1).expect("index 1 should derive");

        assert_ne!(first.address_base58, second.address_base58);
        assert_eq!(second.derivation_path, "m/1");
    }

    #[test]
    fn public_rpc_error_message_is_compact_and_bounded() {
        let long_message = format!(
            "first line\n{}\nlast line",
            "token-like-error-fragment ".repeat(20)
        );
        let message = public_error_message(&long_message);

        assert!(!message.contains('\n'));
        assert!(message.chars().count() <= 163);
        assert!(message.ends_with("..."));
    }

    #[test]
    fn builds_round_two_node_request_from_peer_round_one_payload() {
        let steps = vec![
            step_with_payload(
                "node-a",
                1,
                json!({ "public_package_hex": "node-a-round1" }),
            ),
            step_with_payload(
                "node-b",
                1,
                json!({ "public_package_hex": "node-b-round1" }),
            ),
        ];

        let request =
            build_node_dkg_round_request(&steps, "node-a", 2).expect("request should build");

        assert_eq!(
            request.peer_round1_packages,
            BTreeMap::from([("node-b".to_string(), "node-b-round1".to_string())])
        );
        assert!(request.peer_round2_packages.is_empty());
    }

    #[test]
    fn builds_round_three_node_request_from_peer_payloads() {
        let steps = vec![
            step_with_payload(
                "node-a",
                1,
                json!({ "public_package_hex": "node-a-round1" }),
            ),
            step_with_payload(
                "node-b",
                1,
                json!({ "public_package_hex": "node-b-round1" }),
            ),
            step_with_payload(
                "node-a",
                2,
                json!({ "round2_packages": { "node-b": "node-a-to-node-b" } }),
            ),
            step_with_payload(
                "node-b",
                2,
                json!({ "round2_packages": { "node-a": "node-b-to-node-a" } }),
            ),
        ];

        let request =
            build_node_dkg_round_request(&steps, "node-a", 3).expect("request should build");

        assert_eq!(
            request.peer_round1_packages,
            BTreeMap::from([("node-b".to_string(), "node-b-round1".to_string())])
        );
        assert_eq!(
            request.peer_round2_packages,
            BTreeMap::from([("node-b".to_string(), "node-b-to-node-a".to_string())])
        );
    }

    #[test]
    fn extracts_completed_master_public_key_from_matching_round_three_payloads() {
        let steps = vec![
            step_with_payload(
                "node-a",
                3,
                json!({ "master_public_key_base58": "matching-master-key" }),
            ),
            step_with_payload(
                "node-b",
                3,
                json!({ "master_public_key_base58": "matching-master-key" }),
            ),
        ];

        let master_public_key =
            extract_completed_master_public_key(&steps).expect("master public key should extract");

        assert_eq!(master_public_key, "matching-master-key");
    }

    #[test]
    fn rejects_mismatched_master_public_keys() {
        let steps = vec![
            step_with_payload(
                "node-a",
                3,
                json!({ "master_public_key_base58": "node-a-master-key" }),
            ),
            step_with_payload(
                "node-b",
                3,
                json!({ "master_public_key_base58": "node-b-master-key" }),
            ),
        ];

        let error = extract_completed_master_public_key(&steps)
            .expect_err("mismatched public keys should fail");

        assert!(matches!(error, DkgError::NodeCallFailed(_)));
    }

    fn step(node_id: &str, round: i32, status: &str) -> DkgStepRow {
        DkgStepRow {
            node_id: node_id.to_string(),
            round,
            status: status.to_string(),
            public_payload: None,
        }
    }

    fn step_with_payload(node_id: &str, round: i32, public_payload: Value) -> DkgStepRow {
        DkgStepRow {
            node_id: node_id.to_string(),
            round,
            status: STATUS_COMPLETED.to_string(),
            public_payload: Some(SqlxJson(public_payload)),
        }
    }

    fn signing_step(node_id: &str, round: i32, status: &str) -> SigningStepRow {
        SigningStepRow {
            node_id: node_id.to_string(),
            round,
            status: status.to_string(),
            public_payload: None,
        }
    }

    fn signing_step_with_payload(
        node_id: &str,
        round: i32,
        public_payload: Value,
    ) -> SigningStepRow {
        SigningStepRow {
            node_id: node_id.to_string(),
            round,
            status: STATUS_COMPLETED.to_string(),
            public_payload: Some(SqlxJson(public_payload)),
        }
    }

    fn all_completed_steps() -> Vec<DkgStepRow> {
        NODE_IDS
            .iter()
            .flat_map(|node_id| {
                DKG_ROUNDS
                    .iter()
                    .map(move |round| step(node_id, *round, STATUS_COMPLETED))
            })
            .collect()
    }

    fn sample_master_public_key_base58() -> String {
        let point = Point::<Ed25519>::generator().to_point();

        bs58::encode(point.to_bytes(true).as_bytes()).into_string()
    }
}

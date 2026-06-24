use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::get,
    Json, Router,
};
use coordinator::{router, AppConfig};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn health_route_reports_configured_dependencies() {
    let config = AppConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: "postgres://frost:frost@localhost:5432/frost".to_string(),
        solana_rpc_url: "https://api.devnet.solana.com".to_string(),
        node_a_url: "http://127.0.0.1:18081".to_string(),
        node_b_url: "http://127.0.0.1:18082".to_string(),
    };

    let response = router(config)
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("health route should respond");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_json(response).await;
    assert_eq!(body["service"], "coordinator");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["database_configured"], true);
    assert_eq!(body["solana_rpc_configured"], true);
    assert!(body.get("solana_rpc_url").is_none());
}

#[tokio::test]
async fn node_health_route_reports_reachable_nodes() {
    let node_a_url = spawn_health_server("node-a").await;
    let node_b_url = spawn_health_server("node-b").await;
    let config = AppConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: "postgres://frost:frost@localhost:5432/frost".to_string(),
        solana_rpc_url: "https://api.devnet.solana.com".to_string(),
        node_a_url,
        node_b_url,
    };

    let response = router(config)
        .oneshot(
            Request::builder()
                .uri("/health/nodes")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("node health route should respond");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_json(response).await;
    assert_eq!(body["nodes"][0]["node_id"], "node-a");
    assert_eq!(body["nodes"][0]["reachable"], true);
    assert_eq!(body["nodes"][1]["node_id"], "node-b");
    assert_eq!(body["nodes"][1]["reachable"], true);
}

async fn to_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    serde_json::from_slice(&bytes).expect("body should be json")
}

async fn spawn_health_server(node_id: &'static str) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test listener should bind");
    let address = listener.local_addr().expect("test listener has address");
    let app = Router::new().route(
        "/health",
        get(move || async move {
            Json(serde_json::json!({
                "service": "tss-node",
                "node_id": node_id,
                "status": "ok"
            }))
        }),
    );

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("test server should run");
    });

    format!("http://{address}")
}

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use tss_node::{router, NodeConfig};
use uuid::Uuid;

#[tokio::test]
async fn health_route_reports_node_identity() {
    let config = NodeConfig {
        node_id: "node-a".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8081,
        database_url: "postgres://frost:frost@localhost:5432/frost".to_string(),
        coordinator_url: "http://localhost:8080".to_string(),
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

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let body: Value = serde_json::from_slice(&bytes).expect("body should be json");

    assert_eq!(body["service"], "tss-node");
    assert_eq!(body["node_id"], "node-a");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["database_configured"], true);
    assert_eq!(body["coordinator_url"], "http://localhost:8080");
}

#[tokio::test]
async fn dkg_round_route_returns_public_placeholder_payload() {
    let config = NodeConfig {
        node_id: "node-a".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8081,
        database_url: "postgres://frost:frost@localhost:5432/frost".to_string(),
        coordinator_url: "http://localhost:8080".to_string(),
    };
    let session_id = Uuid::new_v4();
    let uri = format!("/internal/dkg/{session_id}/round1");

    let response = router(config)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("DKG route should respond");

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let body: Value = serde_json::from_slice(&bytes).expect("body should be json");

    assert_eq!(body["session_id"], session_id.to_string());
    assert_eq!(body["node_id"], "node-a");
    assert_eq!(body["round"], 1);
    assert_eq!(body["status"], "COMPLETED");
    assert_eq!(
        body["public_payload"]["kind"],
        "phase-2-placeholder-dkg-round"
    );
    assert!(body.get("root_share").is_none());
    assert!(body.get("nonce_secret").is_none());
    assert!(body["public_payload"].get("root_share").is_none());
    assert!(body["public_payload"].get("nonce_secret").is_none());
}

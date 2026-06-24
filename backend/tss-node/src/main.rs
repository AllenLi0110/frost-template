#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tss_node=info,tower_http=info".into()),
        )
        .init();

    let config = tss_node::NodeConfig::from_env()?;
    tss_node::run(config).await
}

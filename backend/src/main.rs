use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "filedeck_backend=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    filedeck_backend::server::start_server().await
}

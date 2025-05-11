use axum::{routing::get, Router};
use dotenv::dotenv;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=trace", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    dotenv().ok();

    let host: String = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
    let port: String = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr: String = format!("{}:{}", host, port);

    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

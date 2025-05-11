use axum::extract::Query;
use axum::http::StatusCode;
use axum::{routing::get, Json, Router};
use dotenv::dotenv;
use sqlx::{migrate::MigrateDatabase, migrate::Migrator, Sqlite, SqlitePool};
use std::collections::HashMap;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static MIGRATOR: Migrator = sqlx::migrate!();
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
    let db_url: String =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://banana_bread.db".to_string());
    let addr: String = format!("{}:{}", host, port);

    if !Sqlite::database_exists(&db_url).await.unwrap() {
        match Sqlite::create_database(&db_url).await {
            Ok(_) => println!("Created database"),
            Err(e) => panic!("Failed to create database: {}", e),
        }
    }
    let db = SqlitePool::connect(&db_url).await.unwrap();

    run_migrations(&db).await;

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/media_count", get(get_media_for_dir));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn run_migrations(db: &SqlitePool) {
    let previous_migrations: Vec<i64> = sqlx::query_scalar("SELECT version FROM _sqlx_migrations")
        .fetch_all(db)
        .await
        .unwrap_or_default();

    MIGRATOR.run(db).await.unwrap();

    let applied_migrations: Vec<(i64, String)> =
        sqlx::query_as("SELECT version, description FROM _sqlx_migrations")
            .fetch_all(db)
            .await
            .unwrap_or_default();

    for (version, desc) in applied_migrations
        .into_iter()
        .filter(|(v, _)| !previous_migrations.contains(v))
    {
        tracing::info!("Applied migration {} ({})", version, desc);
    }
}

async fn get_media_for_dir(
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<usize>, StatusCode> {
    let dir = if let Some(d) = params.get("dir") {
        d.clone()
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let found_media = crawl_for_media(dir).await;
    Ok(Json(found_media.len()))
}

async fn crawl_for_media(dir: String) -> Vec<Media> {
    let media = vec![];
    return media;
}
struct Media {
    id: String,
    name: String,
    path: String,
    alias: String,
}

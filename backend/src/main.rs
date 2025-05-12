use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{patch, post};
use axum::{debug_handler, routing::get, Json, Router};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{migrate::MigrateDatabase, migrate::Migrator, Sqlite, SqlitePool};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;
use walkdir::WalkDir;

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

    let state = AppState { db };
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/media/scan", post(scan_dir_for_media))
        .route("/media/{file_id}", get(get_media_for_id))
        .route("/media/{file_id}", patch(set_alias_for_media))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
#[debug_handler]
async fn get_media_for_id(
    Path(file_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Media>, StatusCode> {
    let file_id = file_id.to_string();
    let rec = sqlx::query_as!(
        Media,
        "SELECT id, name, path, alias FROM media WHERE id = ?",
        file_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(rec))
}

async fn set_alias_for_media(
    Path(file_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<SetAliasBody>,
) -> Result<StatusCode, StatusCode> {
    let alias = payload.alias;
    let file_id = file_id.to_string();
    let result = sqlx::query!(
        r#"UPDATE media SET alias = $1 WHERE id = $2"#,
        alias,
        file_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update alias for {}: {}", file_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
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

async fn scan_dir_for_media(
    State(state): State<AppState>,
    Json(payload): Json<ScanDirBody>,
) -> Result<Json<usize>, StatusCode> {
    let dir = payload.dir;

    let found_media = crawl_for_media(dir).await;
    _ = insert_media_collection(&found_media, &state.db).await;
    Ok(Json(found_media.len()))
}

async fn insert_media_collection(media: &Vec<Media>, db: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    for item in media {
        sqlx::query("INSERT INTO media (id, name, path, alias) VALUES (?, ?, ?, ?)")
            .bind(&item.id)
            .bind(&item.name)
            .bind(&item.path)
            .bind(&item.alias)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn crawl_for_media(dir: String) -> Vec<Media> {
    let mut media = vec![];
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path_to_entry = entry.path();
        if !path_to_entry.is_file() {
            continue;
        }
        // let file_metadata = fs::metadata(path_to_entry);
        // let file_extension: String = path_to_entry
        //     .extension()
        //     .and_then(|e| e.to_str())
        //     .unwrap_or("")
        //     .to_lowercase();
        let file_name = path_to_entry
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        media.push(Media {
            id: Uuid::new_v4().to_string(),
            name: file_name,
            path: path_to_entry.to_str().unwrap().to_string(),
            alias: Option::from(String::from("")),
        })
    }
    media
}
#[derive(Serialize)]
struct Media {
    id: String,
    name: String,
    path: String,
    alias: Option<String>,
}
#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

#[derive(Deserialize)]
struct ScanDirBody {
    dir: String,
}

#[derive(Deserialize)]
struct SetAliasBody {
    alias: String,
}

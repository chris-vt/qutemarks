use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

mod db;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    bookmarks: Vec<db::Bookmark>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Setup SQLite DB
    let db_url = "sqlite://bookmarks.db";
    let options = SqliteConnectOptions::from_str(db_url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    // Initialize Schema and Seed Fake Data
    db::init_db(&pool).await?;

    let app = Router::new()
        .route("/", get(index))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index(State(pool): State<sqlx::SqlitePool>) -> impl IntoResponse {
    let bookmarks = db::get_all_bookmarks(&pool).await.unwrap_or_default();
    let template = IndexTemplate { bookmarks };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Template error: {}", err),
        )
            .into_response(),
    }
}

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Json, Form, Router,
};
use serde::Deserialize;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

mod db;
mod settings;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    bookmarks: Vec<db::Bookmark>,
    has_untagged: bool,
    normal_tags: Vec<String>,
}

#[derive(Template)]
#[template(path = "edit.html")]
struct EditTemplate {
    bookmark: db::Bookmark,
}

#[derive(Deserialize)]
struct CreateBookmark {
    url: String,
    title: String,
    notes: Option<String>,
}

#[derive(Deserialize)]
struct EditBookmarkForm {
    title: String,
    tags: String,
    notes: String,
}

#[derive(Template)]
#[template(path = "start.html")]
struct StartTemplate {
    pins: Vec<db::PinnedUrl>,
}

#[derive(Deserialize)]
struct AddPinForm {
    url: String,
    title: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let db_url = "sqlite://bookmarks.db";
    let options = SqliteConnectOptions::from_str(db_url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    db::init_db(&pool).await?;

    let app = Router::new()
        .route("/", get(index))
        .route("/api/bookmarks", post(create_bookmark))
        .route("/edit/{id}", get(edit_page))
        .route("/edit/{id}", post(update_bookmark))
        .route("/delete/{id}", post(delete_bookmark))
        .route("/start", get(start_page))
        .route("/start/add", post(add_pin))
        .route("/start/delete/{id}", post(delete_pin))
        .route("/settings", get(settings::settings_page))
        .route("/settings/export/csv", get(settings::export_csv))
        .route("/settings/export/html", get(settings::export_html))
        .route("/settings/backup", get(settings::backup_qutemarks))
        .route("/settings/restore", post(settings::restore_qutemarks))
        .route("/settings/import", post(settings::import_html))
        .route("/settings/delete-all", post(settings::delete_all))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8338").await?;
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index(State(pool): State<sqlx::SqlitePool>) -> impl IntoResponse {
    let bookmarks = db::get_all_bookmarks(&pool).await.unwrap_or_default();
    let all_tags = db::get_all_tags(&pool).await.unwrap_or_default();
    
    let has_untagged = all_tags.contains(&"untagged".to_string());
    let normal_tags = all_tags.into_iter().filter(|t| t != "untagged").collect();
    
    let template = IndexTemplate { bookmarks, has_untagged, normal_tags };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {}", err)).into_response(),
    }
}

async fn create_bookmark(
    State(pool): State<sqlx::SqlitePool>,
    Json(payload): Json<CreateBookmark>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match db::add_bookmark(&pool, &payload.url, &payload.title, payload.notes.as_deref()).await {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn edit_page(
    State(pool): State<sqlx::SqlitePool>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    if let Ok(Some(bookmark)) = db::get_bookmark(&pool, id).await {
        let template = EditTemplate { bookmark };
        match template.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {}", err)).into_response(),
        }
    } else {
        (StatusCode::NOT_FOUND, "Bookmark not found").into_response()
    }
}

async fn update_bookmark(
    State(pool): State<sqlx::SqlitePool>,
    Path(id): Path<i64>,
    Form(form): Form<EditBookmarkForm>,
) -> impl IntoResponse {
    let tags: Vec<String> = form.tags.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let notes = if form.notes.trim().is_empty() { None } else { Some(form.notes.as_str()) };
    
    match db::update_bookmark(&pool, id, &form.title, notes, tags).await {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn delete_bookmark(
    State(pool): State<sqlx::SqlitePool>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match db::delete_bookmark(&pool, id).await {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn start_page(State(pool): State<sqlx::SqlitePool>) -> impl IntoResponse {
    let pins = db::get_all_pins(&pool).await.unwrap_or_default();
    let template = StartTemplate { pins };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {}", err)).into_response(),
    }
}

async fn add_pin(
    State(pool): State<sqlx::SqlitePool>,
    Form(form): Form<AddPinForm>,
) -> impl IntoResponse {
    match db::add_pin(&pool, &form.url, &form.title).await {
        Ok(_) => Redirect::to("/start").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn delete_pin(
    State(pool): State<sqlx::SqlitePool>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match db::delete_pin(&pool, id).await {
        Ok(_) => Redirect::to("/start").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

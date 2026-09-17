use axum::{
    extract::{Multipart, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use askama::Template;
use regex::Regex;

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate {}

pub async fn settings_page() -> impl IntoResponse {
    let template = SettingsTemplate {};
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {}", err)).into_response(),
    }
}

pub async fn export_csv(State(pool): State<sqlx::SqlitePool>) -> impl IntoResponse {
    let bookmarks = crate::db::get_all_bookmarks(&pool).await.unwrap_or_default();
    
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(&["id", "url", "title", "tags", "notes", "created_at"]).unwrap();
    
    for b in bookmarks {
        let tags = b.tags.join(",");
        let notes = b.notes.unwrap_or_default();
        wtr.write_record(&[
            b.id.to_string(),
            b.url,
            b.title,
            tags,
            notes,
            b.created_at.to_string(),
        ]).unwrap();
    }
    
    let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
    
    Response::builder()
        .header(header::CONTENT_TYPE, "text/csv")
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"bookmarks.csv\"")
        .body(data)
        .unwrap()
}

pub async fn export_html(State(pool): State<sqlx::SqlitePool>) -> impl IntoResponse {
    let bookmarks = crate::db::get_all_bookmarks(&pool).await.unwrap_or_default();
    
    let mut html = String::from(
        "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n\
        <META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n\
        <TITLE>Bookmarks</TITLE>\n\
        <H1>Bookmarks</H1>\n\
        <DL><p>\n"
    );
    
    for b in bookmarks {
        let tags = b.tags.join(",");
        let tags_attr = if tags.is_empty() { String::new() } else { format!(" TAGS=\"{}\"", tags) };
        html.push_str(&format!(
            "    <DT><A HREF=\"{}\" ADD_DATE=\"{}\"{}>{}</A>\n",
            b.url, b.created_at, tags_attr, b.title
        ));
        if let Some(notes) = b.notes {
            html.push_str(&format!("    <DD>{}\n", notes));
        }
    }
    html.push_str("</DL><p>\n");
    
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html")
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"bookmarks.html\"")
        .body(html)
        .unwrap()
}

pub async fn delete_all(State(pool): State<sqlx::SqlitePool>) -> impl IntoResponse {
    // Danger Zone!
    let _ = sqlx::query("DELETE FROM bookmark_tags").execute(&pool).await;
    let _ = sqlx::query("DELETE FROM bookmarks").execute(&pool).await;
    // Keep tags table, or clear it if you want
    
    Redirect::to("/").into_response()
}

pub async fn import_html(
    State(pool): State<sqlx::SqlitePool>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if field.name() == Some("file") {
            let data = field.bytes().await.unwrap_or_default();
            if let Ok(text) = String::from_utf8(data.to_vec()) {
                // Extremely basic parsing for first draft
                let re = Regex::new(r#"(?i)<A\s+[^>]*HREF="([^"]+)"[^>]*>([^<]*)</A>"#).unwrap();
                for cap in re.captures_iter(&text) {
                    let url = &cap[1];
                    let title = &cap[2];
                    let _ = crate::db::add_bookmark(&pool, url, title, None).await;
                }
            }
        }
    }
    Redirect::to("/").into_response()
}

pub async fn backup_qutemarks(State(pool): State<sqlx::SqlitePool>) -> impl IntoResponse {
    let bookmarks = crate::db::get_all_bookmarks(&pool).await.unwrap_or_default();
    
    let json_data = serde_json::to_string_pretty(&bookmarks).unwrap_or_default();
    
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"backup.qutemarks\"")
        .body(json_data)
        .unwrap()
}

pub async fn restore_qutemarks(
    State(pool): State<sqlx::SqlitePool>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if field.name() == Some("file") {
            let data = field.bytes().await.unwrap_or_default();
            if let Ok(bookmarks) = serde_json::from_slice::<Vec<crate::db::Bookmark>>(&data) {
                // Wipe DB completely before restore to ensure perfect sync
                let _ = sqlx::query("DELETE FROM bookmark_tags").execute(&pool).await;
                let _ = sqlx::query("DELETE FROM bookmarks").execute(&pool).await;
                
                for b in bookmarks {
                    let _ = crate::db::restore_bookmark(&pool, &b).await;
                }
            }
        }
    }
    Redirect::to("/").into_response()
}

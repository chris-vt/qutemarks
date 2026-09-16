use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Bookmark {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub url: String,
    pub title: String,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub async fn init_db(pool: &SqlitePool) -> anyhow::Result<()> {
    // Create schema
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            parent_id INTEGER,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            FOREIGN KEY (parent_id) REFERENCES folders(id)
        );

        CREATE TABLE IF NOT EXISTS bookmarks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            folder_id INTEGER,
            url TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            notes TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (folder_id) REFERENCES folders(id)
        );

        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS bookmark_tags (
            bookmark_id INTEGER,
            tag_id INTEGER,
            PRIMARY KEY (bookmark_id, tag_id),
            FOREIGN KEY (bookmark_id) REFERENCES bookmarks(id),
            FOREIGN KEY (tag_id) REFERENCES tags(id)
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Seed data if empty
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM bookmarks")
        .fetch_one(pool)
        .await?;

    if count.0 == 0 {
        // Insert sample folders
        sqlx::query(
            r#"
            INSERT INTO folders (name, path) VALUES ('Tech', '/tech');
            INSERT INTO folders (name, path) VALUES ('News', '/news');
            "#,
        )
        .execute(pool)
        .await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        // Insert sample bookmarks
        sqlx::query(
            r#"
            INSERT INTO bookmarks (folder_id, url, title, notes, created_at, updated_at)
            VALUES 
            (1, 'https://github.com/qutebrowser/qutebrowser', 'qutebrowser - github', 'Keyboard-driven browser', ?, ?),
            (1, 'https://doc.rust-lang.org/book/', 'The Rust Programming Language', 'Official Rust Book', ?, ?),
            (2, 'https://news.ycombinator.com', 'Hacker News', 'Tech news', ?, ?),
            (1, 'https://htmx.org', 'htmx - high power tools for HTML', 'Frontend library for MVP', ?, ?);
            "#,
        )
        .bind(now).bind(now)
        .bind(now).bind(now)
        .bind(now).bind(now)
        .bind(now).bind(now)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_all_bookmarks(pool: &SqlitePool) -> anyhow::Result<Vec<Bookmark>> {
    let bookmarks = sqlx::query_as::<_, Bookmark>(
        "SELECT id, folder_id, url, title, notes, created_at, updated_at FROM bookmarks ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(bookmarks)
}

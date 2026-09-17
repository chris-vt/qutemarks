use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub url: String,
    pub title: String,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub tags: Vec<String>,
}

impl Bookmark {
    pub fn domain(&self) -> String {
        if let Ok(parsed) = url::Url::parse(&self.url) {
            parsed.host_str().unwrap_or(&self.url).to_string()
        } else {
            self.url.clone()
        }
    }

    pub fn condensed_notes(&self) -> Option<String> {
        self.notes.as_ref().map(|n| {
            n.split('\n')
                .map(|s| s.trim_end())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        })
    }
}

#[derive(sqlx::FromRow)]
struct BookmarkRow {
    id: i64,
    folder_id: Option<i64>,
    url: String,
    title: String,
    notes: Option<String>,
    created_at: i64,
    updated_at: i64,
    tags_string: Option<String>,
}

impl From<BookmarkRow> for Bookmark {
    fn from(row: BookmarkRow) -> Self {
        let tags = row
            .tags_string
            .map(|s| s.split(',').map(String::from).collect())
            .unwrap_or_default();
        Self {
            id: row.id,
            folder_id: row.folder_id,
            url: row.url,
            title: row.title,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
            tags,
        }
    }
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

        CREATE TABLE IF NOT EXISTS pinned_urls (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            created_at INTEGER NOT NULL
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
        sqlx::query("INSERT INTO tags (name) VALUES ('untagged'), ('rust'), ('browser')").execute(pool).await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        sqlx::query(
            "INSERT INTO bookmarks (id, folder_id, url, title, notes, created_at, updated_at) VALUES 
            (1, NULL, 'https://github.com/qutebrowser/qutebrowser', 'qutebrowser - github', 'Keyboard-driven browser', ?, ?),
            (2, NULL, 'https://doc.rust-lang.org/book/', 'The Rust Programming Language', 'Official Rust Book', ?, ?);"
        ).bind(now).bind(now).bind(now).bind(now).execute(pool).await?;

        sqlx::query("INSERT INTO bookmark_tags (bookmark_id, tag_id) VALUES (1, 3), (2, 2)").execute(pool).await?;
    }

    Ok(())
}

pub async fn get_all_bookmarks(pool: &SqlitePool) -> anyhow::Result<Vec<Bookmark>> {
    let rows = sqlx::query_as::<_, BookmarkRow>(
        r#"
        SELECT b.id, b.folder_id, b.url, b.title, b.notes, b.created_at, b.updated_at,
               GROUP_CONCAT(t.name, ',') as tags_string
        FROM bookmarks b
        LEFT JOIN bookmark_tags bt ON b.id = bt.bookmark_id
        LEFT JOIN tags t ON bt.tag_id = t.id
        GROUP BY b.id
        ORDER BY b.title COLLATE NOCASE ASC
        "#
    )
    .fetch_all(pool)
    .await?;
    
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_all_tags(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    let tags: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT t.name FROM tags t JOIN bookmark_tags bt ON t.id = bt.tag_id ORDER BY t.name ASC"
    )
        .fetch_all(pool)
        .await?;
    Ok(tags.into_iter().map(|(name,)| name).collect())
}

pub async fn get_bookmark(pool: &SqlitePool, id: i64) -> anyhow::Result<Option<Bookmark>> {
    let row = sqlx::query_as::<_, BookmarkRow>(
        r#"
        SELECT b.id, b.folder_id, b.url, b.title, b.notes, b.created_at, b.updated_at,
               GROUP_CONCAT(t.name, ',') as tags_string
        FROM bookmarks b
        LEFT JOIN bookmark_tags bt ON b.id = bt.bookmark_id
        LEFT JOIN tags t ON bt.tag_id = t.id
        WHERE b.id = ?
        GROUP BY b.id
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    
    Ok(row.map(Into::into))
}

pub async fn add_bookmark(pool: &SqlitePool, url: &str, title: &str, notes: Option<&str>) -> anyhow::Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    
    let mut tx = pool.begin().await?;

    // Insert or update bookmark
    sqlx::query(
        r#"
        INSERT INTO bookmarks (url, title, notes, created_at, updated_at) 
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(url) DO UPDATE SET 
            title = excluded.title, 
            notes = excluded.notes, 
            updated_at = excluded.updated_at
        "#
    )
    .bind(url).bind(title).bind(notes).bind(now).bind(now)
    .execute(&mut *tx)
    .await?;

    let bookmark_id: (i64,) = sqlx::query_as("SELECT id FROM bookmarks WHERE url = ?")
        .bind(url)
        .fetch_one(&mut *tx)
        .await?;
    
    // Ensure "untagged" tag exists and get ID
    sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES ('untagged')").execute(&mut *tx).await?;
    let tag_id: (i64,) = sqlx::query_as("SELECT id FROM tags WHERE name = 'untagged'").fetch_one(&mut *tx).await?;

    // Assign "untagged" if it has NO tags at all
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM bookmark_tags WHERE bookmark_id = ?")
        .bind(bookmark_id.0)
        .fetch_one(&mut *tx).await?;
        
    if count.0 == 0 {
        sqlx::query("INSERT INTO bookmark_tags (bookmark_id, tag_id) VALUES (?, ?)")
            .bind(bookmark_id.0)
            .bind(tag_id.0)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn update_bookmark(pool: &SqlitePool, id: i64, title: &str, notes: Option<&str>, tags: Vec<String>) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;

    sqlx::query("UPDATE bookmarks SET title = ?, notes = ?, updated_at = ? WHERE id = ?")
        .bind(title).bind(notes).bind(now).bind(id)
        .execute(&mut *tx).await?;

    // Clear old tags
    sqlx::query("DELETE FROM bookmark_tags WHERE bookmark_id = ?").bind(id).execute(&mut *tx).await?;

    for tag in tags {
        let tag = tag.trim().to_lowercase();
        if tag.is_empty() { continue; }
        sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?)").bind(&tag).execute(&mut *tx).await?;
        let tag_id: (i64,) = sqlx::query_as("SELECT id FROM tags WHERE name = ?").bind(&tag).fetch_one(&mut *tx).await?;
        
        sqlx::query("INSERT INTO bookmark_tags (bookmark_id, tag_id) VALUES (?, ?)")
            .bind(id).bind(tag_id.0).execute(&mut *tx).await?;
    }

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM bookmark_tags WHERE bookmark_id = ?")
        .bind(id)
        .fetch_one(&mut *tx).await?;
        
    if count.0 == 0 {
        sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES ('untagged')").execute(&mut *tx).await?;
        let tag_id: (i64,) = sqlx::query_as("SELECT id FROM tags WHERE name = 'untagged'").fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO bookmark_tags (bookmark_id, tag_id) VALUES (?, ?)")
            .bind(id).bind(tag_id.0).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn delete_bookmark(pool: &SqlitePool, id: i64) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM bookmark_tags WHERE bookmark_id = ?").bind(id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM bookmarks WHERE id = ?").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PinnedUrl {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub created_at: i64,
}

pub async fn get_all_pins(pool: &SqlitePool) -> anyhow::Result<Vec<PinnedUrl>> {
    let rows = sqlx::query_as::<_, PinnedUrl>(
        r#"
        SELECT id, url, title, created_at
        FROM pinned_urls
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(pool)
    .await?;
    
    Ok(rows)
}

pub async fn add_pin(pool: &SqlitePool, url: &str, title: &str) -> anyhow::Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    sqlx::query(
        r#"
        INSERT INTO pinned_urls (url, title, created_at) 
        VALUES (?, ?, ?)
        ON CONFLICT(url) DO UPDATE SET title = excluded.title
        "#
    )
    .bind(url)
    .bind(title)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_pin(pool: &SqlitePool, id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM pinned_urls WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn restore_bookmark(pool: &SqlitePool, b: &Bookmark) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO bookmarks (id, url, title, notes, created_at, updated_at) 
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            url = excluded.url,
            title = excluded.title, 
            notes = excluded.notes, 
            created_at = excluded.created_at,
            updated_at = excluded.updated_at
        "#
    )
    .bind(b.id).bind(&b.url).bind(&b.title).bind(&b.notes).bind(b.created_at).bind(b.updated_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM bookmark_tags WHERE bookmark_id = ?").bind(b.id).execute(&mut *tx).await?;

    for tag in &b.tags {
        let tag = tag.trim().to_lowercase();
        if tag.is_empty() { continue; }
        sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?)").bind(&tag).execute(&mut *tx).await?;
        let tag_id: (i64,) = sqlx::query_as("SELECT id FROM tags WHERE name = ?").bind(&tag).fetch_one(&mut *tx).await?;
        
        sqlx::query("INSERT INTO bookmark_tags (bookmark_id, tag_id) VALUES (?, ?)")
            .bind(b.id).bind(tag_id.0).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

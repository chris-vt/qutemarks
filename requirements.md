# Requirements Specification: Local Bookmark Manager (`bkmrkd`)

## 1. Executive Summary

A lightweight, self-contained local bookmark management service written in Rust. It pairs tightly with keyboard-driven browsers (specifically qutebrowser) via an IPC userscript for quick capture and a fast, local web dashboard for hierarchical browsing, fuzzy searching, and metadata editing.

---

## 2. Core Workflows

```
Capture:   [qutebrowser active tab] -> Press ',b' -> Userscript (POST 127.0.0.1:PORT) -> SQLite -> QUTE_FIFO notification
Browse:    Press ',m' -> Open http://127.0.0.1:PORT -> Tree/Tag view -> qutebrowser hints ('f') open targets
Search:    Focus search ('/' or 's') -> Client-side fuzzy filter -> Instant list pruning
Edit:      Inline edit / modal -> Update title, path, tags, description -> Async PATCH to API

```

---

## 3. Functional Requirements

### 3.1 Ingestion & Browser Integration

* **FR-1.1 (Quick Capture):** Expose an HTTP endpoint `POST /api/bookmarks` accepting JSON `{ "url": string, "title": string, "notes"?: string }`.
* **FR-1.2 (qutebrowser Integration):** Provide a shell script (or small helper CLI) compatible with qutebrowser’s `:spawn --userscript`:
* Read `$QUTE_URL`, `$QUTE_TITLE`, and optional `$QUTE_SELECTED_TEXT`.
* Send payload to daemon.
* Write confirmation or error string to named pipe `$QUTE_FIFO` (e.g., `message-info "Bookmark saved"`).


* **FR-1.3 (Deduplication):** If a URL already exists, update the timestamp and return a 200/204 with an indicator rather than creating duplicate entries.

### 3.2 Web Dashboard & Navigation

* **FR-2.1 (Single Binary Delivery):** Frontend assets (HTML, CSS, minimal JS) embedded directly into the Rust executable at compile-time (`rust-embed`).
* **FR-2.2 (Hint Ergonomics):** All rendered bookmark targets must be standard `<a>` tags with valid `href` attributes to ensure native qutebrowser `f`/`F` hint targeting works without JavaScript interception.
* **FR-2.3 (Folder Tree Hierarchy):** Display a collapsible directory tree based on a virtual path structure (e.g., `/dev/rust/networking`).
* **FR-2.4 (Fuzzy Search):** In-page search bar triggered by hotkey (`/` or `s`) filtering by title, URL, and tag matches instantaneously.
* **FR-2.5 (CRUD Operations):** UI forms to edit title, description/notes, folder path, and tags via async `PATCH` and `DELETE` requests.

---

## 4. Technical Architecture & Tech Stack

* **Language:** Rust (edition 2024 / latest stable).
* **HTTP Framework:** `axum` (built on Tokio, lightweight, idiomatic routing and state management).
* **Database & Persistence:** SQLite via `rusqlite` or `sqlx` (single file on disk, WAL mode enabled for concurrent reads/writes).
* **Asset Embedding:** `rust-embed` (packages `dist/` or static files directly into the compiled artifact).
* **Frontend UI:** Vanilla JS / lightweight framework (e.g., HTMX + Alpine.js, or minimal React/Svelte/Preact) with zero heavy runtime overhead. Client-side fuzzy filtering via `minisearch` or `fuse.js`.

---

## 5. Data Schema

### Table: `folders`

| Column | Type | Constraints | Description |
| --- | --- | --- | --- |
| `id` | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique folder identifier |
| `parent_id` | INTEGER | NULLABLE, FK -> folders(id) | Nested tree pointer |
| `name` | TEXT | NOT NULL | Display name |
| `path` | TEXT | NOT NULL UNIQUE | Materialized path (e.g., `/tech/compilers`) |

### Table: `bookmarks`

| Column | Type | Constraints | Description |
| --- | --- | --- | --- |
| `id` | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique bookmark ID |
| `folder_id` | INTEGER | NULLABLE, FK -> folders(id) | Assigned folder |
| `url` | TEXT | NOT NULL UNIQUE | Canonical target URL |
| `title` | TEXT | NOT NULL | Page title |
| `notes` | TEXT | NULLABLE | Selected text or user notes |
| `created_at` | INTEGER | NOT NULL | Unix epoch seconds |
| `updated_at` | INTEGER | NOT NULL | Unix epoch seconds |

### Table: `tags` & Junction `bookmark_tags`

* `tags(id, name UNIQUE)`
* `bookmark_tags(bookmark_id FK, tag_id FK, PRIMARY KEY (bookmark_id, tag_id))`

---

## 6. API Surface

| Method | Endpoint | Description | Payload |
| --- | --- | --- | --- |
| `POST` | `/api/bookmarks` | Capture new bookmark | `{ url, title, notes?, folder_path?, tags? }` |
| `GET` | `/api/bookmarks` | List bookmarks (supports `?search=`, `?tag=`) | None |
| `PATCH` | `/api/bookmarks/:id` | Update metadata | Partial bookmark fields |
| `DELETE` | `/api/bookmarks/:id` | Remove bookmark | None |
| `GET` | `/api/folders` | Return nested folder hierarchy | None |
| `POST` | `/api/folders` | Create folder path | `{ path: "/dev/rust" }` |

---

## 7. Configuration & Storage Paths

Following XDG base directory standards:

* **Database location:** `$XDG_DATA_HOME/bkmrkd/bookmarks.db` (fallback `~/.local/share/bkmrkd/bookmarks.db`)
* **Server configuration:** `$XDG_CONFIG_HOME/bkmrkd/config.toml` (bind port, host interface default `127.0.0.1:8080`)

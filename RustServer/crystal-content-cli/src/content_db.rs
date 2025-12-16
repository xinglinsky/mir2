use std::path::Path;

use rusqlite::{params, Connection};

pub struct ContentDb {
    conn: Connection,
}

impl ContentDb {
    pub fn open<P: AsRef<Path>>(path: P) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let db = ContentDb { conn };
        db.init_schema()?;
        Ok(db)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn init_schema(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS content_items (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                path          TEXT NOT NULL,
                path_lower    TEXT NOT NULL UNIQUE,
                kind          TEXT NOT NULL,
                enabled       INTEGER NOT NULL DEFAULT 1,
                created_at    INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS content_revisions (
                id               INTEGER PRIMARY KEY AUTOINCREMENT,
                item_id          INTEGER NOT NULL,
                sha256           TEXT NOT NULL,
                original_encoding TEXT NOT NULL,
                content_utf8     TEXT NOT NULL,
                created_at       INTEGER NOT NULL,
                message          TEXT NOT NULL,
                FOREIGN KEY(item_id) REFERENCES content_items(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_content_revisions_item_id_id
            ON content_revisions(item_id, id);

            CREATE TABLE IF NOT EXISTS snapshots (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                note        TEXT NOT NULL,
                created_at  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS snapshot_entries (
                snapshot_id   INTEGER NOT NULL,
                item_id       INTEGER NOT NULL,
                revision_id   INTEGER NOT NULL,
                path          TEXT NOT NULL,
                kind          TEXT NOT NULL,
                sha256        TEXT NOT NULL,
                PRIMARY KEY(snapshot_id, item_id),
                FOREIGN KEY(snapshot_id) REFERENCES snapshots(id) ON DELETE CASCADE,
                FOREIGN KEY(item_id) REFERENCES content_items(id) ON DELETE CASCADE,
                FOREIGN KEY(revision_id) REFERENCES content_revisions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_snapshot_entries_snapshot_id
            ON snapshot_entries(snapshot_id);
            "#,
        )?;

        let _ = self
            .conn
            .execute("ALTER TABLE content_revisions ADD COLUMN is_binary INTEGER NOT NULL DEFAULT 0", []);
        let _ = self
            .conn
            .execute("ALTER TABLE content_revisions ADD COLUMN content_blob BLOB", []);
        let _ = self
            .conn
            .execute("ALTER TABLE content_revisions ADD COLUMN size INTEGER NOT NULL DEFAULT 0", []);

        // One-time best-effort backfill for older rows created before `size` existed.
        // For text rows, use byte length of `content_utf8`. For binary rows, use length of `content_blob`.
        // SQLite doesn't have a strict BOOLEAN type; `is_binary` is 0/1.
        let _ = self.conn.execute(
            "UPDATE content_revisions\n             SET size = LENGTH(content_utf8)\n             WHERE is_binary = 0 AND size = 0 AND content_utf8 IS NOT NULL",
            [],
        );
        let _ = self.conn.execute(
            "UPDATE content_revisions\n             SET size = LENGTH(content_blob)\n             WHERE is_binary = 1 AND size = 0 AND content_blob IS NOT NULL",
            [],
        );
        Ok(())
    }

    pub fn upsert_item(&self, path: &str, path_lower: &str, kind: &str, created_at: i64) -> rusqlite::Result<i64> {
        let mut stmt = self.conn.prepare("SELECT id FROM content_items WHERE path_lower = ?1 LIMIT 1")?;
        let mut rows = stmt.query([path_lower])?;
        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            self.conn.execute(
                "UPDATE content_items SET path = ?1, kind = ?2 WHERE id = ?3",
                params![path, kind, id],
            )?;
            Ok(id)
        } else {
            self.conn.execute(
                "INSERT INTO content_items (path, path_lower, kind, enabled, created_at) VALUES (?1, ?2, ?3, 1, ?4)",
                params![path, path_lower, kind, created_at],
            )?;
            Ok(self.conn.last_insert_rowid())
        }
    }

    pub fn latest_revision_hash(&self, item_id: i64) -> rusqlite::Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT sha256 FROM content_revisions WHERE item_id = ?1 ORDER BY id DESC LIMIT 1",
        )?;
        let mut rows = stmt.query([item_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn insert_revision_text(
        &self,
        item_id: i64,
        sha256: &str,
        original_encoding: &str,
        content_utf8: &str,
        created_at: i64,
        message: &str,
    ) -> rusqlite::Result<i64> {
        let size = content_utf8.as_bytes().len() as i64;
        self.conn.execute(
            "INSERT INTO content_revisions (item_id, sha256, original_encoding, content_utf8, content_blob, is_binary, size, created_at, message)
             VALUES (?1, ?2, ?3, ?4, NULL, 0, ?5, ?6, ?7)",
            params![
                item_id,
                sha256,
                original_encoding,
                content_utf8,
                size,
                created_at,
                message
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_revision_blob(
        &self,
        item_id: i64,
        sha256: &str,
        original_encoding: &str,
        content_blob: &[u8],
        created_at: i64,
        message: &str,
    ) -> rusqlite::Result<i64> {
        let size = content_blob.len() as i64;
        self.conn.execute(
            "INSERT INTO content_revisions (item_id, sha256, original_encoding, content_utf8, content_blob, is_binary, size, created_at, message)
             VALUES (?1, ?2, ?3, '', ?4, 1, ?5, ?6, ?7)",
            params![item_id, sha256, original_encoding, content_blob, size, created_at, message],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn create_snapshot(&self, note: &str, created_at: i64) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO snapshots (note, created_at) VALUES (?1, ?2)",
            params![note, created_at],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn populate_snapshot_with_latest(&self, snapshot_id: i64, include_kinds: &[&str]) -> rusqlite::Result<()> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, kind FROM content_items WHERE enabled = 1",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let path: String = row.get(1)?;
            let kind: String = row.get(2)?;
            Ok((id, path, kind))
        })?;

        for row in rows {
            let (item_id, path, kind) = row?;
            if !include_kinds.iter().any(|k| k.eq_ignore_ascii_case(&kind)) {
                continue;
            }

            let mut stmt2 = self.conn.prepare(
                "SELECT id, sha256 FROM content_revisions WHERE item_id = ?1 ORDER BY id DESC LIMIT 1",
            )?;
            let mut r2 = stmt2.query([item_id])?;
            let Some(rr) = r2.next()? else { continue; };
            let rev_id: i64 = rr.get(0)?;
            let sha256: String = rr.get(1)?;

            self.conn.execute(
                "INSERT OR REPLACE INTO snapshot_entries (snapshot_id, item_id, revision_id, path, kind, sha256) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![snapshot_id, item_id, rev_id, path, kind, sha256],
            )?;
        }

        Ok(())
    }
}

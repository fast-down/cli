use crate::fmt;
use bitcode::{Decode, Encode};
use color_eyre::Result;
use dashmap::DashMap;
use fast_down::FileId;
use rusqlite::{Connection, OpenFlags, params};
use std::{env, ffi::OsStr, fmt::Display, path::PathBuf, sync::Arc, time::Duration};
use url::Url;

const DB_VERSION: u32 = 2;

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct DatabaseEntry {
    pub file_name: String,
    pub file_size: u64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub progress: Vec<(u64, u64)>,
    /// 单位：毫秒
    pub elapsed: u64,
    pub url: String,
}

impl Display for DatabaseEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}: {}", t!("db-display.file-name"), self.file_name)?;
        writeln!(
            f,
            "{}: {}",
            t!("db-display.size"),
            fmt::format_size(self.file_size as f64)
        )?;
        writeln!(f, "{}: {:?}", t!("db-display.etag"), self.etag)?;
        writeln!(
            f,
            "{}: {:?}",
            t!("db-display.last-modified"),
            self.last_modified
        )?;
        write!(f, "{}: ", t!("db-display.progress"))?;
        for (i, (start, end)) in self.progress.iter().enumerate() {
            write!(f, "{}-{}", start, end - 1)?;
            if i < self.progress.len() - 1 {
                write!(f, ", ")?;
            }
        }
        writeln!(f)?;
        writeln!(
            f,
            "{}: {:?}",
            t!("db-display.elapsed"),
            Duration::from_millis(self.elapsed)
        )?;
        writeln!(f, "{}: {}", t!("db-display.url"), self.url)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Database {
    db_path: PathBuf,
    cache: Arc<DashMap<String, (bool, DatabaseEntry)>>,
}

impl Display for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let conn = Connection::open_with_flags(&self.db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| std::fmt::Error)?;
        let version: u32 = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'version'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        writeln!(f, "{}: {}", t!("db-display.version"), version)?;
        writeln!(f, "---")?;
        let mut stmt = conn
            .prepare("SELECT path, data FROM downloads")
            .map_err(|_| std::fmt::Error)?;
        let rows = stmt
            .query_map([], |row| {
                let path: String = row.get(0)?;
                let data: Vec<u8> = row.get(1)?;
                Ok((path, data))
            })
            .map_err(|_| std::fmt::Error)?;
        for row in rows.flatten() {
            if let Ok(entry) = bitcode::decode::<DatabaseEntry>(&row.1) {
                writeln!(f, "{}: {}", t!("db-display.file-path"), row.0)?;
                writeln!(f, "{}", entry)?;
            }
        }
        Ok(())
    }
}

impl Database {
    pub async fn new() -> Result<Self> {
        let db_path = env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_owned()))
            .unwrap_or(PathBuf::from("."))
            .join("fdstate.db");
        let conn = Connection::open(&db_path)?;
        conn.execute("PRAGMA busy_timeout = 5000;", [])?;
        conn.execute("PRAGMA journal_mode = WAL;", [])?;
        conn.execute("PRAGMA synchronous = NORMAL;", [])?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value INTEGER)",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS downloads (path TEXT PRIMARY KEY, data BLOB)",
            [],
        )?;
        let instance = Self {
            db_path: db_path.clone(),
            cache: Arc::new(DashMap::new()),
        };
        instance.check_version_and_init(&conn)?;
        let flush_cache = instance.cache.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let _ = Self::static_flush(&db_path, &flush_cache);
            }
        });
        Ok(instance)
    }

    fn check_version_and_init(&self, conn: &Connection) -> Result<()> {
        let current_version: u32 = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'version'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if current_version != DB_VERSION {
            conn.execute("DELETE FROM downloads", [])?;
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES ('version', ?)",
                params![DB_VERSION],
            )?;
        }
        Ok(())
    }

    fn static_flush(path: &PathBuf, cache: &DashMap<String, (bool, DatabaseEntry)>) -> Result<()> {
        let mut dirty_items = Vec::new();
        for mut r in cache.iter_mut() {
            if r.0 {
                dirty_items.push((r.key().clone(), bitcode::encode(&r.1)));
                r.0 = false;
            }
        }
        if dirty_items.is_empty() {
            return Ok(());
        }
        let mut conn = Connection::open(path)?;
        conn.execute("PRAGMA busy_timeout = 5000;", [])?;
        let tx = conn.transaction()?;
        {
            let mut stmt =
                tx.prepare("INSERT OR REPLACE INTO downloads (path, data) VALUES (?, ?)")?;
            for (path, data) in dirty_items {
                stmt.execute(params![path, data])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn init_entry(
        &self,
        file_path: impl AsRef<OsStr>,
        file_name: String,
        file_size: u64,
        file_id: &FileId,
        url: Url,
    ) -> Result<()> {
        let path_str = file_path.as_ref().to_string_lossy().to_string();
        let entry = DatabaseEntry {
            file_name,
            file_size,
            etag: file_id.etag.as_ref().map(|s| s.to_string()),
            last_modified: file_id.last_modified.as_ref().map(|s| s.to_string()),
            url: url.to_string(),
            progress: Vec::new(),
            elapsed: 0,
        };
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO downloads (path, data) VALUES (?, ?)",
            params![path_str, bitcode::encode(&entry)],
        )?;
        self.cache.insert(path_str, (false, entry));
        Ok(())
    }

    pub fn get_entry(&self, file_path: impl AsRef<OsStr>) -> Option<DatabaseEntry> {
        let path_str = file_path.as_ref().to_string_lossy();
        if let Some(e) = self.cache.get(path_str.as_ref()) {
            return Some(e.1.clone());
        }
        let conn = Connection::open(&self.db_path).ok()?;
        let data: Vec<u8> = conn
            .query_row(
                "SELECT data FROM downloads WHERE path = ?",
                params![path_str.as_ref()],
                |r| r.get(0),
            )
            .ok()?;
        let entry: DatabaseEntry = bitcode::decode(&data).ok()?;
        self.cache
            .insert(path_str.to_string(), (false, entry.clone()));
        Some(entry)
    }

    pub fn update_entry(
        &self,
        file_path: impl AsRef<OsStr>,
        progress: Vec<(u64, u64)>,
        elapsed: u64,
    ) {
        let path_str = file_path.as_ref().to_string_lossy();
        if let Some(mut e) = self.cache.get_mut(path_str.as_ref()) {
            e.0 = true;
            e.1.progress = progress;
            e.1.elapsed = elapsed;
        } else if let Some(mut entry) = self.get_entry(file_path.as_ref()) {
            entry.progress = progress;
            entry.elapsed = elapsed;
            self.cache.insert(path_str.to_string(), (true, entry));
        }
    }

    pub fn remove_entry(&self, file_path: impl AsRef<OsStr>) -> Result<()> {
        let path_str = file_path.as_ref().to_string_lossy();
        self.cache.remove(path_str.as_ref());
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "DELETE FROM downloads WHERE path = ?",
            params![path_str.as_ref()],
        )?;
        Ok(())
    }

    pub fn flush_force(&self) -> Result<()> {
        Self::static_flush(&self.db_path, &self.cache)
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = self.flush_force();
    }
}

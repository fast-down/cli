use crate::fmt;
use bitcode::{Decode, Encode};
use color_eyre::Result;
use dashmap::DashMap;
use fast_down::FileId;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use std::{
    env,
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::fs;
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
    conn: Arc<Mutex<Connection>>,
    cache: Arc<DashMap<String, (bool, DatabaseEntry)>>,
}

impl Display for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}: {}", t!("db-display.version"), DB_VERSION)?;
        writeln!(f, "---")?;
        let conn = self.conn.lock();
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
    async fn setup_db(path: &Path) -> Result<Arc<Mutex<Connection>>> {
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_millis(5000))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS downloads (path TEXT PRIMARY KEY, data BLOB)",
            [],
        )?;

        let mut paths_to_delete = Vec::new();
        {
            let mut stmt = conn.prepare("SELECT path FROM downloads")?;
            let path_iter = stmt.query_map([], |row| {
                let p: String = row.get(0)?;
                Ok(p)
            })?;
            for path in path_iter {
                let path = path?;
                if !fs::try_exists(&path).await.unwrap_or(false) {
                    paths_to_delete.push(path);
                }
            }
        }
        if !paths_to_delete.is_empty() {
            let tx = conn.transaction()?;
            {
                let mut del_stmt = tx.prepare("DELETE FROM downloads WHERE path = ?")?;
                for p in paths_to_delete {
                    del_stmt.execute([p])?;
                }
            }
            tx.commit()?;
        }

        Ok(Arc::new(Mutex::new(conn)))
    }

    pub async fn new() -> Result<Self> {
        let db_path = env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_owned()))
            .unwrap_or(PathBuf::from("."))
            .join(format!("fd-state-v{}.db", DB_VERSION));
        let conn = Self::setup_db(&db_path).await?;
        let cache = Arc::new(DashMap::new());
        let conn_weak = Arc::downgrade(&conn);
        let cache_weak = Arc::downgrade(&cache);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let conn = conn_weak.upgrade().unwrap();
                let cache = cache_weak.upgrade().unwrap();
                let _ =
                    tokio::task::spawn_blocking(move || Self::static_flush(&conn, &cache)).await;
            }
        });
        let instance = Self { cache, conn };
        Ok(instance)
    }

    fn static_flush(
        conn: &Mutex<Connection>,
        cache: &DashMap<String, (bool, DatabaseEntry)>,
    ) -> Result<()> {
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
        let mut conn = conn.lock();
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
        let conn = self.conn.lock();
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
        let conn = self.conn.lock();
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
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM downloads WHERE path = ?",
            params![path_str.as_ref()],
        )?;
        Ok(())
    }

    pub fn flush_force(&self) -> Result<()> {
        Self::static_flush(&self.conn, &self.cache)
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = self.flush_force();
    }
}

use crate::fmt::add_prefix_to_lines;
use crate::model::downloading::Downloading;
use color_eyre::Result;
use dashmap::DashMap;
use fast_down::FileId;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::fmt::Write;
use std::path::Path;
use std::{env, ffi::OsStr, path::PathBuf, sync::Arc, time::Duration};
use tokio::fs;
use url::Url;

const CURRENT_DB_VERSION: u32 = 2;

#[derive(Debug, Clone)]
pub struct Store {
    db: Arc<Mutex<Connection>>,
    db_path: PathBuf,
    /// file_path: record
    cache: Arc<DashMap<String, (bool, Downloading)>>,
}

impl Store {
    async fn setup_db(path: &Path) -> Result<Arc<Mutex<Connection>>> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_millis(5000))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS downloads (path TEXT PRIMARY KEY, data BLOB)",
            [],
        )?;

        Ok(Arc::new(Mutex::new(conn)))
    }

    pub async fn new() -> Result<Self> {
        let db_path = env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_owned()))
            .unwrap_or(PathBuf::from("."))
            .join(format!("fd-state-v{}.db", CURRENT_DB_VERSION));

        let store_instance = Self {
            db: Self::setup_db(&db_path).await?,
            cache: Arc::new(DashMap::new()),
            db_path,
        };

        store_instance.clean().await?;

        let db_weak = Arc::downgrade(&store_instance.db);
        let cache_weak = Arc::downgrade(&store_instance.cache);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                let Some(conn) = db_weak.upgrade() else {
                    return;
                };
                let Some(cache) = cache_weak.upgrade() else {
                    return;
                };

                let _ =
                    tokio::task::spawn_blocking(move || Self::static_flush(&conn, &cache)).await;
            }
        });

        Ok(store_instance)
    }

    pub async fn clean(&self) -> Result<()> {
        let mut paths_to_delete = Vec::new();
        let paths: Vec<String> = {
            let conn = self.db.lock();
            let mut stmt = conn.prepare("SELECT path FROM downloads")?;
            let path_iter = stmt.query_map([], |row| {
                let p: String = row.get(0)?;
                Ok(p)
            })?;
            let mut paths = Vec::new();
            for path in path_iter {
                paths.push(path?);
            }
            paths
        };

        for path in paths {
            if !fs::try_exists(&path).await.unwrap_or(false) {
                paths_to_delete.push(path);
            }
        }

        if !paths_to_delete.is_empty() {
            let mut conn = self.db.lock();
            let tx = conn.transaction()?;
            {
                let mut del_stmt = tx.prepare("DELETE FROM downloads WHERE path = ?")?;
                for p in paths_to_delete {
                    del_stmt.execute([p])?;
                }
            }
            tx.commit()?;
        };
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
        let entry = Downloading {
            file_name,
            file_size,
            etag: file_id.etag.as_ref().map(|s| s.to_string()),
            last_modified: file_id.last_modified.as_ref().map(|s| s.to_string()),
            url: url.to_string(),
            progress: Vec::new(),
            elapsed: Duration::ZERO,
        };

        let conn = self.db.lock();
        conn.execute(
            "INSERT OR REPLACE INTO downloads (path, data) VALUES (?, ?)",
            params![path_str, entry.dump()],
        )?;
        self.cache.insert(path_str, (false, entry));
        Ok(())
    }

    pub fn get_all_entry(&self) -> Option<DashMap<String, Downloading>> {
        let entries = DashMap::new();
        let db = self.db.lock();

        let mut stmt = db.prepare("SELECT path, data FROM downloads").ok()?;
        let rows = stmt
            .query_map([], |row| {
                let path: String = row.get(0)?;
                let data: Vec<u8> = row.get(1)?;
                Ok((path, data))
            })
            .ok()?;
        for row in rows.flatten() {
            entries.insert(row.0, Downloading::load(&row.1)?);
        }
        if entries.is_empty() {
            return None;
        }

        Some(entries)
    }

    pub fn get_entry(&self, file_path: impl AsRef<OsStr>) -> Option<Downloading> {
        let path_str = file_path.as_ref().to_string_lossy();
        if let Some(e) = self.cache.get(path_str.as_ref()) {
            return Some(e.1.clone());
        }
        let conn = self.db.lock();
        let data: Vec<u8> = conn
            .query_row(
                "SELECT data FROM downloads WHERE path = ?",
                params![path_str.as_ref()],
                |r| r.get(0),
            )
            .ok()?;
        let entry = Downloading::load(&data)?;
        self.cache
            .insert(path_str.to_string(), (false, entry.clone()));
        Some(entry)
    }

    pub fn update_entry(
        &self,
        file_path: impl AsRef<OsStr>,
        progress: Vec<(u64, u64)>,
        elapsed: Duration,
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
        let conn = self.db.lock();
        conn.execute(
            "DELETE FROM downloads WHERE path = ?",
            params![path_str.as_ref()],
        )?;
        Ok(())
    }

    fn static_flush(
        conn: &Mutex<Connection>,
        cache: &DashMap<String, (bool, Downloading)>,
    ) -> Result<()> {
        let mut dirty_items = Vec::new();
        for mut r in cache.iter_mut() {
            if r.0 {
                dirty_items.push((r.key().clone(), r.1.dump())); // file_path: dump(record)
                r.0 = false; // stored (false = flushed, true = to flush)
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

    pub fn force_flush(&self) -> Result<()> {
        Self::static_flush(&self.db, &self.cache)
    }

    pub fn display(&self, with_details: bool) -> std::result::Result<String, std::fmt::Error> {
        let mut content = String::new();

        if with_details {
            writeln!(
                &mut content,
                "{}: {}",
                t!("db-display.version"),
                CURRENT_DB_VERSION
            )?;
            if let Some(db_path) = self.db_path.as_os_str().to_str() {
                writeln!(&mut content, "{}: {}", t!("db-display.db-path"), db_path)?;
            }
        }

        writeln!(&mut content, "----------------------")?;

        let Some(entries) = self.get_all_entry() else {
            return Ok(content);
        };

        for ref_multi in entries.iter() {
            let path: &String = ref_multi.key();
            let entry: &Downloading = ref_multi.value();

            write!(&mut content, "| ")?;
            writeln!(&mut content, "{}: {}", t!("db-display.file-path"), path)?;

            let downloading_display = entry.display(with_details)?;
            let downloading_display = add_prefix_to_lines(&downloading_display, "| ");
            writeln!(&mut content, "{}", downloading_display)?;

            writeln!(&mut content, "----------------------")?;
        }

        Ok(content)
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _ = self.force_flush();
    }
}

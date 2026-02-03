use crate::fmt;
use bitcode::{Decode, Encode};
use color_eyre::Result;
use fast_down::{FileId, ProgressEntry};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{fs, time::Instant};
use url::Url;

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
pub type DatabaseTable = HashMap<String, DatabaseEntry>;

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

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct DatabaseInner(/* version */ u8, DatabaseTable);

#[derive(Debug, Clone)]
pub struct Database {
    inner: Arc<Mutex<DatabaseInner>>,
    db_path: Arc<PathBuf>,
    last_db_update: Arc<AtomicU64>,
    init: Instant,
}

impl Display for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let guard = self.inner.lock();
        writeln!(f, "{}: {}", t!("db-display.version"), DB_VERSION)?;
        writeln!(f, "---")?;
        for (file_path, entry) in &guard.1 {
            writeln!(f, "{}: {}", t!("db-display.file-path"), file_path)?;
            writeln!(f, "{}", entry)?;
        }
        Ok(())
    }
}

const DB_VERSION: u8 = 2;

impl Database {
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut DatabaseTable) -> R,
    {
        let mut guard = self.inner.lock();
        f(&mut guard.1)
    }

    pub async fn new() -> Result<Self> {
        let db_path = env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_owned()))
            .unwrap_or(PathBuf::from("."))
            .join("state.fd");
        if fs::try_exists(&db_path).await? {
            match Self::from_file(&db_path).await {
                Ok(Some(db)) => return Ok(db),
                Ok(None) => eprintln!("{}", t!("err.database-version")),
                Err(err) => eprintln!("{}: {:#?}", t!("err.database-load"), err),
            };
        }
        Ok(Self {
            inner: Arc::new(Mutex::new(DatabaseInner(DB_VERSION, HashMap::new()))),
            db_path: Arc::new(db_path),
            init: Instant::now(),
            last_db_update: Arc::new(AtomicU64::new(0)),
        })
    }

    pub async fn from_file(file_path: impl AsRef<Path>) -> Result<Option<Self>> {
        let bytes = fs::read(&file_path).await?;
        let mut archived: DatabaseInner = bitcode::decode(&bytes)?;
        if archived.0 != DB_VERSION {
            return Ok(None);
        }
        archived
            .1
            .retain(|file_path, _| Path::new(file_path).try_exists().unwrap_or(false));
        Ok(Some(Self {
            inner: Arc::new(Mutex::new(archived)),
            db_path: Arc::new(file_path.as_ref().to_path_buf()),
            init: Instant::now(),
            last_db_update: Arc::new(AtomicU64::new(0)),
        }))
    }

    pub async fn init_entry(
        &self,
        file_path: impl AsRef<OsStr>,
        file_name: String,
        file_size: u64,
        file_id: &FileId,
        url: Url,
    ) -> Result<()> {
        let file_path = file_path.as_ref().to_string_lossy();
        self.with(|db| {
            db.retain(|key, _| key != &file_path);
            db.insert(
                file_path.to_string(),
                DatabaseEntry {
                    file_name,
                    file_size,
                    etag: file_id.etag.as_ref().map(|s| s.to_string()),
                    last_modified: file_id.last_modified.as_ref().map(|s| s.to_string()),
                    url: url.to_string(),
                    progress: Vec::new(),
                    elapsed: 0,
                },
            );
        });
        self.flush().await?;
        Ok(())
    }

    pub fn get_entry(&self, file_path: impl AsRef<OsStr>) -> Option<DatabaseEntry> {
        let file_path = file_path.as_ref().to_string_lossy();
        self.inner.lock().1.get(file_path.as_ref()).cloned()
    }

    pub async fn update_entry(
        &self,
        file_path: impl AsRef<OsStr>,
        progress: Vec<ProgressEntry>,
        elapsed: u64,
    ) -> Result<()> {
        let file_path = file_path.as_ref().to_string_lossy();
        self.with(|db| {
            if let Some(entry) = db.get_mut(file_path.as_ref()) {
                entry.progress = progress.iter().map(|r| (r.start, r.end)).collect();
                entry.elapsed = elapsed;
            }
        });
        self.flush().await?;
        Ok(())
    }

    pub async fn remove_entry(&self, file_path: impl AsRef<OsStr>) -> Result<()> {
        let file_path = file_path.as_ref().to_string_lossy();
        self.with(|db| {
            db.remove(file_path.as_ref());
        });
        self.flush().await?;
        Ok(())
    }

    pub async fn flush(&self) -> Result<()> {
        let now = Instant::now().duration_since(self.init).as_secs();
        let old = self.last_db_update.load(Ordering::Acquire);
        if now - old > 1 {
            let bytes = bitcode::encode(&*self.inner.lock());
            fs::write(&*self.db_path, bytes).await?;
            self.last_db_update.store(now, Ordering::Release);
        }
        Ok(())
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let bytes = bitcode::encode(&*self.inner.lock());
        let _ = std::fs::write(&*self.db_path, bytes);
    }
}

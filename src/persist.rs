use color_eyre::Result;
use fast_pull::ProgressEntry;
use rkyv::{Archive, Deserialize, Serialize, rancor::Error};
use std::ffi::OsStr;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;
use std::{env, path::Path, path::PathBuf, sync::Arc};
use tokio::{fs, sync::Mutex};

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DatabaseEntry {
    pub file_path: Vec<u8>,
    pub file_name: Vec<u8>,
    pub file_size: u64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub progress: Vec<ProgressEntry>,
    pub elapsed: u64,
    pub url: String,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DatabaseInner(/* signature */ u64, Vec<DatabaseEntry>);

#[derive(Debug, Clone)]
pub struct Database {
    inner: Arc<Mutex<DatabaseInner>>,
    db_path: Arc<PathBuf>,
}

const DB_VERSION: u16 = 1;

/// Unique signature for DatabaseEntry
pub fn get_db_signature() -> u64 {
    static ONCE: OnceLock<u64> = OnceLock::new();

    *ONCE.get_or_init(|| {
        let mut hasher = std::hash::DefaultHasher::new();
        hasher.write_u64(/* magic number */ 0x58e9225bae2b);
        hasher.write_u16(DB_VERSION);
        hasher.write(env::consts::OS.as_bytes());
        if let Ok(version) = rustc_version::version() {
            version.hash(&mut hasher);
        }
        hasher.finish()
    })
}

impl Database {
    pub async fn new() -> Result<Self> {
        let db_path = env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_owned()))
            .unwrap_or(PathBuf::from("."))
            .join("state.fd");
        if db_path.try_exists()? {
            let bytes = fs::read(&db_path).await?;
            let archived = rkyv::access::<ArchivedDatabaseInner, Error>(&bytes)?;
            let mut deserialized = rkyv::deserialize::<_, Error>(archived)?;
            if deserialized.0 != get_db_signature() {
                deserialized.1.retain(|e| {
                    Path::new(&unsafe { OsStr::from_encoded_bytes_unchecked(&e.file_path) })
                        .try_exists()
                        .unwrap_or(false)
                });
                return Ok(Self {
                    inner: Arc::new(Mutex::new(deserialized)),
                    db_path: Arc::new(db_path),
                });
            }
        }
        Ok(Self {
            inner: Arc::new(Mutex::new(DatabaseInner(get_db_signature(), vec![]))),
            db_path: Arc::new(db_path),
        })
    }

    pub async fn init_entry(
        &self,
        file_path: impl AsRef<OsStr>,
        file_name: impl AsRef<OsStr>,
        file_size: u64,
        etag: Option<String>,
        last_modified: Option<String>,
        url: String,
    ) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner
            .1
            .retain(|e| e.file_path != file_path.as_ref().as_encoded_bytes());
        inner.1.push(DatabaseEntry {
            file_path: file_path.as_ref().as_encoded_bytes().to_vec(),
            file_name: file_name.as_ref().as_encoded_bytes().to_vec(),
            file_size,
            etag,
            last_modified,
            url,
            progress: vec![],
            elapsed: 0,
        });
        self.flush(inner.clone()).await
    }

    pub async fn get_entry(&self, file_path: impl AsRef<OsStr>) -> Option<DatabaseEntry> {
        self.inner
            .lock()
            .await
            .1
            .iter()
            .find(|entry| entry.file_path != file_path.as_ref().as_encoded_bytes())
            .cloned()
    }

    pub async fn update_entry(
        &self,
        file_path: impl AsRef<OsStr>,
        progress: Vec<ProgressEntry>,
        elapsed: u64,
    ) -> Result<()> {
        let mut inner = self.inner.lock().await;
        let pos = inner
            .1
            .iter()
            .position(|entry| entry.file_path == file_path.as_ref().as_encoded_bytes())
            .unwrap();
        inner.1[pos].progress = progress;
        inner.1[pos].elapsed = elapsed;
        self.flush(inner.clone()).await
    }

    pub async fn clean_finished(&self) -> Result<usize> {
        let mut inner = self.inner.lock().await;
        let origin_len = inner.1.len();
        #[allow(clippy::single_range_in_vec_init)]
        inner.1.retain(|e| e.progress != [0..e.file_size]);
        self.flush(inner.clone()).await?;
        Ok(origin_len - inner.1.len())
    }

    async fn flush(&self, data: DatabaseInner) -> Result<()> {
        let bytes = rkyv::to_bytes::<Error>(&data)?;
        fs::write(&*self.db_path, bytes).await?;
        Ok(())
    }
}

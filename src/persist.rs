use color_eyre::Result;
use fast_pull::ProgressEntry;
use rkyv::{Archive, Deserialize, Serialize, rancor::Error};
use std::ffi::OsStr;
use std::{env, path::Path, path::PathBuf, sync::Arc};
use tokio::{fs, sync::Mutex};

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DatabaseEntry {
    pub file_path: Vec<u8>,
    pub file_name: String,
    pub file_size: u64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub progress: Vec<ProgressEntry>,
    pub elapsed: u64,
    pub url: String,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DatabaseInner(/* version */ u16, Vec<DatabaseEntry>);

#[derive(Debug, Clone)]
pub struct Database {
    inner: Arc<Mutex<DatabaseInner>>,
    db_path: Arc<PathBuf>,
}

const DB_VERSION: u16 = 1;

impl Database {
    pub async fn new() -> Result<Self> {
        let db_path = env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_owned()))
            .unwrap_or(PathBuf::from("."))
            .join("state.fd");
        if db_path.try_exists()? {
            match Self::from_file(&db_path).await {
                Ok(Some(db)) => return Ok(db),
                Ok(None) => eprintln!("{}", t!("err.database-version")),
                Err(err) => eprintln!("{}: {:#?}", t!("err.database-load"), err),
            };
        }
        Ok(Self {
            inner: Arc::new(Mutex::new(DatabaseInner(DB_VERSION, vec![]))),
            db_path: Arc::new(db_path),
        })
    }

    pub async fn from_file(file_path: impl AsRef<Path>) -> Result<Option<Self>> {
        let bytes = fs::read(&file_path).await?;
        let archived = rkyv::access::<ArchivedDatabaseInner, Error>(&bytes)?;
        let mut deserialized = rkyv::deserialize::<_, Error>(archived)?;
        if deserialized.0 != DB_VERSION {
            return Ok(None);
        }
        deserialized.1.retain(|e| {
            Path::new(&unsafe { OsStr::from_encoded_bytes_unchecked(&e.file_path) })
                .try_exists()
                .unwrap_or(false)
        });
        Ok(Some(Self {
            inner: Arc::new(Mutex::new(deserialized)),
            db_path: Arc::new(file_path.as_ref().to_path_buf()),
        }))
    }

    pub async fn init_entry(
        &self,
        file_path: impl AsRef<OsStr>,
        file_name: String,
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
            file_name,
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
            .find(|entry| entry.file_path == file_path.as_ref().as_encoded_bytes())
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

use crate::fmt;
use bitcode::{Decode, Encode};
use color_eyre::Result;
use fast_down::{FileId, ProgressEntry};
use redb::{
    Database as RedbDatabase, ReadableDatabase, ReadableTable, TableDefinition, TypeName, Value,
};
use std::{env, ffi::OsStr, fmt::Display, path::PathBuf, sync::Arc, time::Duration};
use tokio::fs;
use url::Url;

const TABLE_DATA: TableDefinition<&str, DatabaseEntry> = TableDefinition::new("downloads");
const TABLE_META: TableDefinition<&str, u32> = TableDefinition::new("metadata");
const KEY_VERSION: &str = "version";
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

impl Value for DatabaseEntry {
    type SelfType<'a> = DatabaseEntry;
    type AsBytes<'a> = Vec<u8>;
    fn fixed_width() -> Option<usize> {
        None
    }
    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bitcode::decode(data).expect("Database corruption: failed to decode DatabaseEntry")
    }
    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a> {
        bitcode::encode(value)
    }
    fn type_name() -> TypeName {
        TypeName::new("DatabaseEntry")
    }
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
    inner: Arc<RedbDatabase>,
}

impl Display for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let read_txn = self.inner.begin_read().map_err(|_| std::fmt::Error)?;
        let version = read_txn
            .open_table(TABLE_META)
            .ok()
            .and_then(|t| t.get(KEY_VERSION).ok().flatten())
            .map(|v| v.value())
            .unwrap_or(0);
        writeln!(f, "{}: {}", t!("db-display.version"), version)?;
        writeln!(f, "---")?;
        if let Ok(table) = read_txn.open_table(TABLE_DATA)
            && let Ok(iter) = table.iter()
        {
            for (key, value) in iter.flatten() {
                writeln!(f, "{}: {}", t!("db-display.file-path"), key.value())?;
                writeln!(f, "{}", value.value())?;
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
            .join("state.fd");

        let db = match RedbDatabase::create(&db_path) {
            Ok(db) => db,
            Err(_) => {
                let _ = fs::remove_file(&db_path).await;
                RedbDatabase::create(&db_path)?
            }
        };
        let instance = Self {
            inner: Arc::new(db),
        };

        instance.check_version_and_init().await?;
        Ok(instance)
    }

    async fn check_version_and_init(&self) -> Result<()> {
        let write_txn = self.inner.begin_write()?;
        {
            let meta_table = write_txn.open_table(TABLE_META)?;
            let current_version = meta_table.get(KEY_VERSION)?.map(|v| v.value());
            if current_version != Some(DB_VERSION) {
                drop(meta_table);
                let _ = write_txn.delete_table(TABLE_DATA);
                let mut meta_table = write_txn.open_table(TABLE_META)?;
                meta_table.insert(KEY_VERSION, DB_VERSION)?;
            }
        }
        write_txn.commit()?;
        Ok(())
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
        let entry = DatabaseEntry {
            file_name,
            file_size,
            etag: file_id.etag.as_ref().map(|s| s.to_string()),
            last_modified: file_id.last_modified.as_ref().map(|s| s.to_string()),
            url: url.to_string(),
            progress: Vec::new(),
            elapsed: 0,
        };
        let write_txn = self.inner.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE_DATA)?;
            table.insert(file_path.as_ref(), &entry)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_entry(&self, file_path: impl AsRef<OsStr>) -> Option<DatabaseEntry> {
        let file_path = file_path.as_ref().to_string_lossy();
        let read_txn = self.inner.begin_read().ok()?;
        let table = read_txn.open_table(TABLE_DATA).ok()?;
        table
            .get(file_path.as_ref())
            .ok()
            .flatten()
            .map(|v| v.value())
    }

    pub async fn update_entry(
        &self,
        file_path: impl AsRef<OsStr>,
        progress: Vec<ProgressEntry>,
        elapsed: u64,
    ) -> Result<()> {
        let file_path_str = file_path.as_ref().to_string_lossy();
        let write_txn = self.inner.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE_DATA)?;
            if let Some(mut guard) = table.get_mut(file_path_str.as_ref())? {
                let mut entry = guard.value();
                entry.progress = progress.iter().map(|r| (r.start, r.end)).collect();
                entry.elapsed = elapsed;
                guard.insert(entry)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    pub async fn remove_entry(&self, file_path: impl AsRef<OsStr>) -> Result<()> {
        let file_path = file_path.as_ref().to_string_lossy();
        let write_txn = self.inner.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE_DATA)?;
            table.remove(file_path.as_ref())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

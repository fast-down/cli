use crate::fmt;
use bitcode::{Decode, Encode};
use color_eyre::Result;
use dashmap::DashMap;
use fast_down::FileId;
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
    cache: Arc<DashMap<String, (bool, DatabaseEntry)>>,
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
        let db = RedbDatabase::create(&db_path)?;
        let db = Arc::new(db);
        let cache = Arc::new(DashMap::new());
        let instance = Self {
            inner: db.clone(),
            cache: cache.clone(),
        };
        instance.check_version_and_init()?;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let _ = Self::static_flush(&db, &cache);
            }
        });
        Ok(instance)
    }

    fn check_version_and_init(&self) -> Result<()> {
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

    pub fn init_entry(
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
        self.cache.insert(file_path.to_string(), (false, entry));
        Ok(())
    }

    pub fn get_entry(&self, file_path: impl AsRef<OsStr>) -> Option<DatabaseEntry> {
        let file_path = file_path.as_ref().to_string_lossy();
        if let Some(e) = self.cache.get(file_path.as_ref()) {
            return Some(e.1.clone());
        }
        let read_txn = self.inner.begin_read().ok()?;
        let table = read_txn.open_table(TABLE_DATA).ok()?;
        let val = table.get(file_path.as_ref()).ok().flatten()?.value();
        self.cache
            .insert(file_path.to_string(), (false, val.clone()));
        Some(val)
    }

    pub fn update_entry(
        &self,
        file_path: impl AsRef<OsStr>,
        progress: Vec<(u64, u64)>,
        elapsed: u64,
    ) {
        let file_path = file_path.as_ref().to_string_lossy();
        if let Some(mut e) = self.cache.get_mut(file_path.as_ref()) {
            e.0 = true;
            e.1.progress = progress;
            e.1.elapsed = elapsed;
        }
    }

    pub fn remove_entry(&self, file_path: impl AsRef<OsStr>) -> Result<()> {
        let file_path = file_path.as_ref().to_string_lossy();
        self.cache.remove(file_path.as_ref());
        let write_txn = self.inner.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE_DATA)?;
            table.remove(file_path.as_ref())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn flush_force(&self) -> Result<()> {
        Self::static_flush(&self.inner, &self.cache)
    }

    fn static_flush(
        inner: &RedbDatabase,
        cache: &DashMap<String, (bool, DatabaseEntry)>,
    ) -> Result<()> {
        let mut dirty_items = Vec::new();
        for mut r in cache.iter_mut() {
            if r.0 {
                dirty_items.push((r.key().clone(), r.1.clone()));
                r.0 = false;
            }
        }
        if dirty_items.is_empty() {
            return Ok(());
        }
        let write_txn = inner.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE_DATA)?;
            for (path, entry) in dirty_items {
                table.insert(path.as_str(), &entry)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = self.flush_force();
    }
}

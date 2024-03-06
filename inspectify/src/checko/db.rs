use std::{
    marker::PhantomData,
    path::Path,
    sync::{Arc, Mutex},
};

use ce_shell::Input;
use color_eyre::eyre::Context;
use driver::JobKind;
use rusqlite::{types::FromSql, OptionalExtension, ToSql};

use crate::endpoints::InspectifyJobMeta;

#[derive(Clone)]
pub struct CheckoDb {
    conn: Arc<Mutex<rusqlite::Connection>>,
}
pub struct Compressed<T> {
    data: Vec<u8>,
    _ph: PhantomData<T>,
}

impl FromSql for Compressed<JobData> {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self {
            data: FromSql::column_result(value)?,
            _ph: PhantomData,
        })
    }
}

impl ToSql for Compressed<JobData> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        self.data.to_sql()
    }
}

impl<T: serde::Serialize + for<'a> serde::Deserialize<'a>> Compressed<T> {
    pub fn compress(data: &T) -> Self {
        let data = serde_json::to_vec(data).unwrap();
        let data = lz4_flex::compress_prepend_size(&data);
        Self {
            data,
            _ph: PhantomData,
        }
    }
    #[tracing::instrument(skip_all)]
    pub fn decompress(&self) -> T {
        let data = lz4_flex::decompress_size_prepended(&self.data).unwrap();
        serde_json::from_slice(&data).unwrap()
    }
}

pub struct Id<T> {
    pub id: usize,
    _ph: PhantomData<T>,
}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Id")
            .field(&std::any::type_name::<T>())
            .field(&self.id)
            .finish()
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> FromSql for Id<T> {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self {
            id: FromSql::column_result(value)?,
            _ph: PhantomData,
        })
    }
}

impl<T> ToSql for Id<T> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        self.id.to_sql()
    }
}

pub struct WithId<T> {
    pub id: Id<T>,
    data: T,
}

impl<T> std::ops::Deref for WithId<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Clone)]
pub struct Run<T = JobData> {
    pub group_name: String,
    input_md5: [u8; 16],
    pub data: T,
    queued: chrono::DateTime<chrono::Utc>,
    started: Option<chrono::DateTime<chrono::Utc>>,
    finished: Option<chrono::DateTime<chrono::Utc>>,
}

pub type JobData = driver::JobData<InspectifyJobMeta>;

pub type CompressedRun = Run<Compressed<JobData>>;

impl From<Run> for CompressedRun {
    fn from(run: Run) -> Self {
        let data = Compressed::compress(&run.data);
        Self {
            group_name: run.group_name,
            input_md5: run.input_md5,
            data,
            queued: run.queued,
            started: run.started,
            finished: run.finished,
        }
    }
}

impl From<CompressedRun> for Run {
    fn from(run: CompressedRun) -> Self {
        let data = run.data.decompress();
        Self {
            group_name: run.group_name,
            input_md5: run.input_md5,
            data,
            queued: run.queued,
            started: run.started,
            finished: run.finished,
        }
    }
}

impl Run {
    pub fn new(group_name: String, input: Input) -> color_eyre::Result<Self> {
        let input_md5 = input.hash();
        Ok(Self {
            group_name: group_name.clone(),
            input_md5,
            data: JobData::new(
                JobKind::Analysis(input),
                InspectifyJobMeta {
                    group_name: Some(group_name),
                },
            ),
            queued: chrono::Utc::now(),
            started: None,
            finished: None,
        })
    }
}

impl Run {
    pub fn input(&self) -> Option<Input> {
        match &self.data.kind {
            JobKind::Analysis(input) => Some(input.clone()),
            _ => None,
        }
    }
}

impl CheckoDb {
    pub fn open(path: &Path) -> color_eyre::Result<Self> {
        tracing::debug!(?path, "opening db");

        let conn = rusqlite::Connection::open(path)?;

        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS cached_runs (
                cache_key TEXT PRIMARY KEY,
                data BLOB NOT NULL
            );
            "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn conn(&self) -> std::sync::MutexGuard<rusqlite::Connection> {
        self.conn.lock().unwrap()
    }

    pub fn get_cached_run(&self, key: &CacheKey) -> color_eyre::Result<Option<JobData>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT data FROM cached_runs WHERE cache_key = ?1")?;
        let run = stmt
            .query_row([&key.0], |row| {
                let data: Compressed<JobData> = row.get(0)?;
                Ok(data.decompress())
            })
            .optional()?;
        Ok(run)
    }

    pub fn insert_cached_run(&self, key: &CacheKey, data: &JobData) -> color_eyre::Result<()> {
        if let Some(prev) = self.get_cached_run(key)? {
            if prev != *data {
                tracing::error!(
                    "cached run for git_hash: {:?}, input: {:?} already exists but with different data", key.1.git_hash, key.1.input);
            }
            return Ok(());
        }
        let data = Compressed::compress(data);
        // TODO: Fix multiple repos being at the same git-hash causing a unique constraint violation
        self.conn()
            .execute(
                "INSERT INTO cached_runs (cache_key, data) VALUES (?1, ?2)",
                (&key.0, data),
            )
            .wrap_err_with(|| {
                format!(
                    "could not insert cached run for git_hash: {:?}, input: {:?}",
                    key.1.git_hash, key.1.input
                )
            })?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct CacheKeyInput<'a> {
    pub group_name: &'a str,
    pub git_hash: &'a str,
    pub input: &'a Input,
}

pub struct CacheKey<'a>(String, CacheKeyInput<'a>);

impl<'a> CacheKeyInput<'a> {
    pub fn key(self) -> CacheKey<'a> {
        CacheKey(
            format!(
                "{}:{}:{:?}",
                self.group_name,
                self.git_hash,
                self.input.hash()
            ),
            self,
        )
    }
}

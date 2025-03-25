use std::{
    borrow::Cow,
    path::Path,
    sync::{Arc, Mutex},
};

use ce_shell::Input;
use color_eyre::eyre::Context;
use rusqlite::OptionalExtension;

use crate::endpoints::InspectifyJobMeta;

use super::{compression::Compressed, config::GroupConfig};

#[derive(Clone)]
pub struct CheckoDb {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

#[derive(Clone)]
pub struct Run {
    pub group_config: Arc<GroupConfig>,
    pub data: JobData,
}

pub type JobData = driver::JobData<InspectifyJobMeta>;

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
                    "cached run for git_hash: {:?}, input: {:?} already exists but with different data",
                    key.1.git_hash,
                    key.1.input,
                );
            }
            return Ok(());
        }
        let data = Compressed::compress(data);
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

#[derive(Clone)]
struct CowCacheKeyInput<'a> {
    group_name: Cow<'a, str>,
    git_hash: Cow<'a, str>,
    input: Cow<'a, Input>,
}

pub struct CacheKey<'a>(String, CowCacheKeyInput<'a>);

impl<'a> CacheKeyInput<'a> {
    pub fn key(self) -> CacheKey<'a> {
        CacheKey(
            format!(
                "{}:{}:{:?}",
                self.group_name,
                self.git_hash,
                self.input.hash()
            ),
            CowCacheKeyInput {
                group_name: Cow::Borrowed(self.group_name),
                git_hash: Cow::Borrowed(self.git_hash),
                input: Cow::Borrowed(self.input),
            },
        )
    }
}

impl CacheKey<'_> {
    pub fn into_owned(self) -> CacheKey<'static> {
        CacheKey(
            self.0,
            CowCacheKeyInput {
                group_name: Cow::Owned(self.1.group_name.into_owned()),
                git_hash: Cow::Owned(self.1.git_hash.into_owned()),
                input: Cow::Owned(self.1.input.into_owned()),
            },
        )
    }
}

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use ce_shell::Input;
use color_eyre::eyre::Context;
use driver::JobKind;
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

impl Run {
    pub fn new(group_config: Arc<GroupConfig>, input: Input) -> color_eyre::Result<Self> {
        let data = JobData::new(
            JobKind::Analysis(input),
            InspectifyJobMeta {
                group_name: Some(group_config.name.clone()),
            },
        );
        Ok(Self { group_config, data })
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

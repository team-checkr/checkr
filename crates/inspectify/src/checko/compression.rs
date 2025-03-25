use std::marker::PhantomData;

use rusqlite::{ToSql, types::FromSql};

pub struct Compressed<T> {
    data: Vec<u8>,
    _ph: PhantomData<T>,
}

impl<T> FromSql for Compressed<T> {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self {
            data: FromSql::column_result(value)?,
            _ph: PhantomData,
        })
    }
}

impl<T> ToSql for Compressed<T> {
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

use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stringify<T: FromStr + Display>(T);

impl<T> Stringify<T>
where
    T: FromStr + Display,
{
    pub fn new(t: T) -> Self {
        Self(t)
    }
    pub fn inner(&self) -> &T {
        &self.0
    }
}

impl<T: FromStr + Display> Serialize for Stringify<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de, T: FromStr + Display> Deserialize<'de> for Stringify<T> {
    fn deserialize<D>(deserializer: D) -> Result<Stringify<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match T::from_str(&s) {
            Ok(t) => Ok(Stringify(t)),
            Err(_) => Err(serde::de::Error::custom("failed to parse")),
        }
    }
}

impl<T: FromStr + Display + 'static> tapi::Tapi for Stringify<T> {
    fn name() -> &'static str {
        std::any::type_name::<T>()
    }

    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<Stringify<T>>()
    }

    fn dependencies() -> Vec<&'static dyn tapi::Typed> {
        vec![]
    }

    fn path() -> Vec<&'static str> {
        vec![]
    }

    fn ts_name() -> String {
        "string".to_string()
    }

    fn zod_name() -> String {
        "z.string()".to_string()
    }
}

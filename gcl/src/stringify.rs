use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stringify<T: FromStr + Display> {
    Parsed(T),
    Unparsed(String),
}

impl<T> Stringify<T>
where
    T: FromStr + Display,
{
    pub fn new(t: T) -> Self {
        Self::Parsed(t)
    }
    pub fn try_parse(&self) -> Result<T, <T as FromStr>::Err>
    where
        T: Clone,
    {
        match self {
            Self::Parsed(t) => Ok(t.clone()),
            Self::Unparsed(s) => T::from_str(s),
        }
    }
}

impl<T: FromStr + Display> Serialize for Stringify<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Parsed(t) => serializer.serialize_str(&t.to_string()),
            Self::Unparsed(s) => serializer.serialize_str(s),
        }
    }
}

impl<'de, T: FromStr + Display> Deserialize<'de> for Stringify<T> {
    fn deserialize<D>(deserializer: D) -> Result<Stringify<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::Unparsed(s))
    }
}

impl<T: FromStr + Display + 'static> tapi::Tapi for Stringify<T> {
    fn name() -> &'static str {
        std::any::type_name::<T>()
    }

    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<Stringify<T>>()
    }

    fn path() -> Vec<&'static str> {
        vec![]
    }

    fn kind() -> tapi::kind::TypeKind {
        tapi::kind::TypeKind::Builtin(tapi::kind::BuiltinTypeKind::String)
    }
}

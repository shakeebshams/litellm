use litellm_python_compat::{Value, pydantic::lax_int};

use super::record::Field;
use crate::{DecodeError, FromValue};

/// A JSON object the model types as a `dict`, kept as the plain data it was read as. It may hold
/// credentials, so `Debug` shows none of it.
#[derive(Clone, Default, PartialEq, veil::Redact)]
pub struct Bag(#[redact(with = "[REDACTED]")] Vec<(Value, Value)>);

impl Bag {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn entries(&self) -> &[(Value, Value)] {
        &self.0
    }
}

/// A `dict[str, list[str]]`, in the order it was written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lists(pub Vec<(String, Vec<String>)>);

/// A `dict[str, int]`, in the order it was written, each value read as pydantic's lax `int`
/// reads it, so `100.0` and `"7"` are `100` and `7`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ints(pub Vec<(String, i64)>);

impl Field for Bag {
    fn read(value: &Value) -> Option<Self> {
        match value {
            Value::Dict(entries) => Some(Self(entries.clone())),
            _ => None,
        }
    }

    fn write(self) -> Value {
        Value::Dict(self.0)
    }
}

impl Field for Lists {
    fn read(value: &Value) -> Option<Self> {
        let Value::Dict(entries) = value else {
            return None;
        };
        entries
            .iter()
            .map(|(name, list)| Some((String::read(name)?, Vec::read(list)?)))
            .collect::<Option<_>>()
            .map(Self)
    }

    fn write(self) -> Value {
        Value::Dict(
            self.0
                .into_iter()
                .map(|(name, list)| (Value::Str(name), list.write()))
                .collect(),
        )
    }
}

impl Field for Ints {
    fn read(value: &Value) -> Option<Self> {
        let Value::Dict(entries) = value else {
            return None;
        };
        entries
            .iter()
            .map(|(name, number)| {
                Some((String::read(name)?, i64::try_from(lax_int(number)?).ok()?))
            })
            .collect::<Option<_>>()
            .map(Self)
    }

    fn write(self) -> Value {
        Value::Dict(
            self.0
                .into_iter()
                .map(|(name, number)| (Value::Str(name), Value::from(number)))
                .collect(),
        )
    }
}

impl FromValue for Bag {
    fn from_value(value: &Value) -> Result<Self, DecodeError> {
        Self::read(value).ok_or(DecodeError::Shape("a dict"))
    }
}

impl FromValue for Lists {
    fn from_value(value: &Value) -> Result<Self, DecodeError> {
        Self::read(value).ok_or(DecodeError::Shape("a dict of string lists"))
    }
}

impl FromValue for Ints {
    fn from_value(value: &Value) -> Result<Self, DecodeError> {
        Self::read(value).ok_or(DecodeError::Shape("a dict of ints"))
    }
}

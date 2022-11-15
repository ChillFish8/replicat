use rusqlite::types::{ToSqlOutput, Value};
use rusqlite::ToSql;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportableParam {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
}

impl ToSql for TransportableParam {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let val = match self {
            TransportableParam::Null => Value::Null,
            TransportableParam::Bool(v) => Value::from(*v),
            TransportableParam::I64(v) => Value::from(*v),
            TransportableParam::F64(v) => Value::from(*v),
            TransportableParam::String(v) => Value::Text(v.clone()),
            TransportableParam::Bytes(v) => Value::Blob(v.clone()),
        };

        Ok(ToSqlOutput::Owned(val))
    }
}

impl From<bool> for TransportableParam {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i64> for TransportableParam {
    fn from(v: i64) -> Self {
        Self::I64(v)
    }
}

impl From<i32> for TransportableParam {
    fn from(v: i32) -> Self {
        Self::I64(v as i64)
    }
}

impl From<f64> for TransportableParam {
    fn from(v: f64) -> Self {
        Self::F64(v)
    }
}

impl From<f32> for TransportableParam {
    fn from(v: f32) -> Self {
        Self::F64(v as f64)
    }
}

impl From<String> for TransportableParam {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<Vec<u8>> for TransportableParam {
    fn from(v: Vec<u8>) -> Self {
        Self::Bytes(v)
    }
}

impl From<&[u8]> for TransportableParam {
    fn from(v: &[u8]) -> Self {
        Self::Bytes(v.to_vec())
    }
}

impl<T: Into<TransportableParam> + Clone> From<&T> for TransportableParam {
    fn from(val: &T) -> Self {
        val.clone().into()
    }
}

pub trait IntoTransportableParams {
    fn to_params(self) -> Vec<TransportableParam>;
}

impl IntoTransportableParams for () {
    fn to_params(self) -> Vec<TransportableParam> {
        Vec::new()
    }
}

impl<T: Into<TransportableParam>, const N: usize> IntoTransportableParams for [T; N] {
    fn to_params(self) -> Vec<TransportableParam> {
        let mut params = Vec::with_capacity(self.len());

        for entry in self {
            params.push(entry.into());
        }

        params
    }
}

impl<T: Into<TransportableParam> + Clone> IntoTransportableParams for &[T] {
    fn to_params(self) -> Vec<TransportableParam> {
        let mut params = Vec::with_capacity(self.len());

        for entry in self {
            params.push(entry.into());
        }

        params
    }
}

impl<T: Into<TransportableParam>> IntoTransportableParams for Vec<T> {
    fn to_params(self) -> Vec<TransportableParam> {
        let mut params = Vec::with_capacity(self.len());

        for entry in self {
            params.push(entry.into());
        }

        params
    }
}

macro_rules! derive_tuple {
    ($($index:tt => $field:ident)*) => {
        impl<$($field: Into<TransportableParam>,)*> IntoTransportableParams for ($($field,)*) {
            fn to_params(self) -> Vec<TransportableParam> {
                vec![
                    $(self.$index.into(),)*
                ]
            }
        }
    }
}

macro_rules! derive_common_tuples {
    () => {};
    ($first_index:tt => $first:ident $($rest_index:tt => $rest:ident)*) => {
        derive_tuple!($first_index => $first $($rest_index => $rest)*);
        derive_common_tuples!($($rest_index => $rest)*);
    };
}

derive_common_tuples! {
    15 => T16
    14 => T15
    13 => T14
    12 => T13
    11 => T12
    10 => T11
    9 => T10
    8 => T9
    7 => T8
    6 => T7
    5 => T6
    4 => T5
    3 => T4
    2 => T3
    1 => T2
    0 => T1
}

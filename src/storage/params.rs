use rusqlite::ToSql;
use rusqlite::types::{ToSqlOutput, Value};
use serde::{Serialize, Deserialize};

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
use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEntry {
    Set { key: String, value: Vec<u8> },
    SetMany { keys: Vec<String>, values: Vec<u8> },
    Delete { key: String },
    DeleteMany { keys: Vec<String> },
}



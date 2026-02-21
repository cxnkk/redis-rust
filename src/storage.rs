use crate::Command;
use crate::resp::RespValue;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct DbEntry {
    data: DbData,
    expires_at: Option<Instant>,
}

pub enum DbData {
    String(String),
    List(Vec<String>),
}

pub type Db = Arc<Mutex<HashMap<String, DbEntry>>>;

pub fn execute_command(cmd: Command, db: &Db) -> RespValue {
    match cmd {
        Command::Ping(msg) => match msg {
            Some(m) => RespValue::BulkString(m),
            None => RespValue::SimpleString("PONG".to_string()),
        },
        Command::Echo(msg) => RespValue::BulkString(msg),
        Command::Set(key, val, px) => {
            let mut map = db.lock().unwrap();
            let expires_at = px.map(|ms| Instant::now() + std::time::Duration::from_millis(ms));

            map.insert(
                key,
                DbEntry {
                    data: DbData::String(val),
                    expires_at,
                },
            );
            RespValue::SimpleString("OK".to_string())
        }
        Command::Get(key) => {
            let mut map = db.lock().unwrap();

            if let Some(entry) = map.get(&key) {
                if let Some(expiry) = entry.expires_at {
                    if Instant::now() > expiry {
                        map.remove(&key);
                        return RespValue::Null;
                    }
                }

                match &entry.data {
                    DbData::String(s) => RespValue::BulkString(s.clone()),
                    DbData::List(_) => RespValue::Error(
                        "WRONGTYPE Operation against a key holding the wrong kind of value"
                            .to_string(),
                    ),
                }
            } else {
                RespValue::Null
            }
        }
        Command::RPush(key, values) => {
            let mut map = db.lock().unwrap();

            let entry = map.entry(key).or_insert(DbEntry {
                data: DbData::List(Vec::new()),
                expires_at: None,
            });

            if let DbData::List(ref mut list) = entry.data {
                for val in values {
                    list.push(val);
                }
                RespValue::Integer(list.len() as i64)
            } else {
                RespValue::Error(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                )
            }
        }
        Command::LRange(key, (start, stop)) => {
            let mut map = db.lock().unwrap();

            let entry = map.entry(key).or_insert(DbEntry {
                data: DbData::List(Vec::new()),
                expires_at: None,
            });

            let mut elems_of_list: Vec<RespValue> = Vec::new();

            if let DbData::List(ref list) = entry.data {
                if list.is_empty() {
                    return RespValue::Array(elems_of_list);
                } else if start >= list.len() {
                    return RespValue::Array(elems_of_list);
                } else if stop >= list.len() {
                    let elems = list[start..].to_vec();
                    for elem in elems {
                        elems_of_list.push(RespValue::BulkString(elem));
                    }
                } else {
                    let elems = list[start..stop + 1].to_vec();
                    for elem in elems {
                        elems_of_list.push(RespValue::BulkString(elem));
                    }
                }
            }
            RespValue::Array(elems_of_list)
        }
    }
}

pub fn extract_string(elems: &[RespValue], index: usize) -> Option<String> {
    match elems.get(index) {
        Some(RespValue::BulkString(s)) => Some(s.clone()),
        _ => None,
    }
}

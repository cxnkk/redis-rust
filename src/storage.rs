use crate::Command;
use crate::resp::RespValue;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct DbValue {
    value: String,
    expires_at: Option<Instant>,
}

pub type Db = Arc<Mutex<HashMap<String, DbValue>>>;

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
                DbValue {
                    value: val,
                    expires_at,
                },
            );
            RespValue::SimpleString("OK".to_string())
        }
        Command::Get(key) => {
            let mut map = db.lock().unwrap();

            if let Some(db_val) = map.get(&key) {
                if let Some(expiry) = db_val.expires_at {
                    if Instant::now() > expiry {
                        map.remove(&key);
                        return RespValue::Null;
                    }
                }
                return RespValue::BulkString(db_val.value.clone());
            }
            RespValue::Null
        }
    }
}

pub fn extract_string(elems: &[RespValue], index: usize) -> Option<String> {
    match elems.get(index) {
        Some(RespValue::BulkString(s)) => Some(s.clone()),
        _ => None,
    }
}

use crate::Command;
use crate::resp::RespValue;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<HashMap<String, String>>>;

pub fn execute_command(cmd: Command, db: &Db) -> RespValue {
    match cmd {
        Command::Ping(msg) => match msg {
            Some(m) => RespValue::BulkString(m),
            None => RespValue::SimpleString("PONG".to_string()),
        },
        Command::Echo(msg) => RespValue::BulkString(msg),
        Command::Set(key, val) => {
            let mut map = db.lock().unwrap();
            map.insert(key, val);
            RespValue::SimpleString("OK".to_string())
        }
        Command::Get(key) => {
            let map = db.lock().unwrap();
            match map.get(&key) {
                Some(val) => RespValue::BulkString(val.clone()),
                None => RespValue::Null,
            }
        }
    }
}

pub fn extract_string(elems: &[RespValue], index: usize) -> Option<String> {
    match elems.get(index) {
        Some(RespValue::BulkString(s)) => Some(s.clone()),
        _ => None,
    }
}

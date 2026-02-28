use crate::Command;
use crate::resp::RespValue;

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

#[derive(Debug)]
pub struct DbEntry {
    data: DbData,
    expires_at: Option<Instant>,
}

#[derive(Debug)]
pub enum DbData {
    String(String),
    List(Vec<String>),
}

pub type Db = Arc<(Mutex<HashMap<String, DbEntry>>, Condvar)>;

pub fn execute_command(cmd: Command, db: &Db) -> RespValue {
    let (lock, cvar) = &**db;

    match cmd {
        Command::Ping(msg) => match msg {
            Some(m) => RespValue::BulkString(m),
            None => RespValue::SimpleString("PONG".to_string()),
        },
        Command::Echo(msg) => RespValue::BulkString(msg),
        Command::Set(key, val, px) => {
            let mut map = lock.lock().unwrap();
            let expires_at = px.map(|ms| Instant::now() + std::time::Duration::from_millis(ms));

            map.insert(
                key,
                DbEntry {
                    data: DbData::String(val),
                    expires_at,
                },
            );
            cvar.notify_all();
            RespValue::SimpleString("OK".to_string())
        }
        Command::Get(key) => {
            let mut map = lock.lock().unwrap();

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
            let mut map = lock.lock().unwrap();

            let entry = map.entry(key).or_insert(DbEntry {
                data: DbData::List(Vec::new()),
                expires_at: None,
            });

            if let DbData::List(ref mut list) = entry.data {
                for val in values {
                    list.push(val);
                }
                cvar.notify_all();
                RespValue::Integer(list.len() as i64)
            } else {
                RespValue::Error(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                )
            }
        }
        Command::LPush(key, values) => {
            let mut map = lock.lock().unwrap();

            let entry = map.entry(key).or_insert(DbEntry {
                data: DbData::List(Vec::new()),
                expires_at: None,
            });

            if let DbData::List(ref mut list) = entry.data {
                for val in values {
                    list.push(val);
                }
                list.reverse();
                RespValue::Integer(list.len() as i64)
            } else {
                RespValue::Error(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                )
            }
        }
        Command::LRange(key, (start, stop)) => {
            let map = lock.lock().unwrap();

            let list = match map.get(&key) {
                Some(entry) => match &entry.data {
                    DbData::List(l) => l,
                    _ => {
                        return RespValue::Error(
                            "WRONGTYPE Operation against a key holding the wrong kind of value"
                                .to_string(),
                        );
                    }
                },
                None => return RespValue::Array(vec![]),
            };

            let len = list.len() as isize;

            let mut start_idx = if start < 0 { len + start } else { start };
            let mut stop_idx = if stop < 0 { len + stop } else { stop };

            if start_idx < 0 {
                start_idx = 0;
            }
            if stop_idx >= len {
                stop_idx = len - 1;
            }

            if start_idx >= len || start_idx > stop_idx {
                return RespValue::Array(vec![]);
            }

            let result = list[start_idx as usize..=stop_idx as usize]
                .iter()
                .map(|s| RespValue::BulkString(s.clone()))
                .collect();

            RespValue::Array(result)
        }
        Command::LLen(key) => {
            let mut map = lock.lock().unwrap();

            let entry = map.entry(key).or_insert(DbEntry {
                data: DbData::List(Vec::new()),
                expires_at: None,
            });

            if let DbData::List(ref mut list) = entry.data {
                RespValue::Integer(list.len() as i64)
            } else {
                RespValue::Error(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                )
            }
        }
        Command::LPop(key, count) => {
            let mut map = lock.lock().unwrap();

            let entry = match map.get_mut(&key) {
                Some(e) => e,
                None => return RespValue::Null,
            };

            if let DbData::List(ref mut list) = entry.data {
                if list.is_empty() {
                    return RespValue::Null;
                }

                match count {
                    None => {
                        let val = list.remove(0);
                        if list.is_empty() {
                            map.remove(&key);
                        }
                        RespValue::BulkString(val)
                    }
                    Some(n) => {
                        let take_n = std::cmp::min(n, list.len());
                        let removed_elements: Vec<RespValue> = list
                            .drain(0..take_n)
                            .map(|s| RespValue::BulkString(s))
                            .collect();

                        if list.is_empty() {
                            map.remove(&key);
                        }
                        RespValue::Array(removed_elements)
                    }
                }
            } else {
                RespValue::Error(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                )
            }
        }
        Command::BLPop(key, timeout) => {
            let mut map = lock.lock().unwrap();

            let timeout_duration = std::time::Duration::from_secs_f32(timeout);
            let start_time = std::time::Instant::now();

            loop {
                if let Some(entry) = map.get_mut(&key) {
                    match &mut entry.data {
                        DbData::List(list) => {
                            if !list.is_empty() {
                                let val = list.remove(0);
                                let result = vec![
                                    RespValue::BulkString(key.clone()),
                                    RespValue::BulkString(val),
                                ];

                                return RespValue::Array(result);
                            }
                        }
                        _ => {
                            return RespValue::Error(
                                "WRONGTYPE Operation against a key holding the wrong kind of value"
                                    .to_string(),
                            );
                        }
                    }
                }

                if timeout == 0.0 {
                    map = cvar.wait(map).unwrap();
                } else {
                    let elapsed = start_time.elapsed();

                    if elapsed >= timeout_duration {
                        return RespValue::Null;
                    }

                    let remaining = timeout_duration - elapsed;

                    let (new_map, result) = cvar.wait_timeout(map, remaining).unwrap();
                    map = new_map;

                    if result.timed_out() {
                        return RespValue::NullArray;
                    }
                }
            }
        }
        Command::Type(key) => {
            let map = lock.lock().unwrap();

            if let Some(entry) = map.get(&key) {
                match &entry.data {
                    DbData::String(_) => RespValue::SimpleString("string".to_string()),
                    DbData::List(_) => RespValue::SimpleString(
                        "WRONGTYPE Operation against a key holding the wrong kind of value"
                            .to_string(),
                    ),
                }
            } else {
                RespValue::SimpleString("none".to_string())
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

#![allow(unused_imports)]
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

type Db = Arc<Mutex<HashMap<String, String>>>;

#[derive(Debug)]
enum Command {
    Ping(Option<String>),
    Echo(String),
    Set(String, String),
    Get(String),
}

impl Command {
    fn from_resp(resp: RespValue) -> Result<Self, String> {
        if let RespValue::Array(elems) = resp {
            let cmd_name = match elems.get(0) {
                Some(RespValue::BulkString(s)) => s.to_uppercase(),
                _ => return Err("Invalid command format".to_string()),
            };

            match cmd_name.as_str() {
                "PING" => {
                    let arg = elems.get(1).and_then(|e| {
                        if let RespValue::BulkString(s) = e {
                            Some(s.clone())
                        } else {
                            None
                        }
                    });
                    Ok(Self::Ping(arg))
                }
                "ECHO" => match elems.get(1) {
                    Some(RespValue::BulkString(s)) => Ok(Self::Echo(s.clone())),
                    _ => Err("ECHO requires an argument".to_string()),
                },
                "SET" => {
                    let key = extract_string(&elems, 1).ok_or("SET missing key")?;
                    let val = extract_string(&elems, 2).ok_or("SET missing value")?;
                    Ok(Self::Set(key, val))
                }
                "GET" => match elems.get(1) {
                    Some(RespValue::BulkString(s)) => Ok(Self::Get(s.clone())),
                    _ => Err("GET requires a key".to_string()),
                },
                _ => Err(format!("Unknown command: {}", cmd_name)),
            }
        } else {
            Err("Expected array of bulk strings".to_string())
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
enum RespValue {
    SimpleString(String),
    BulkString(String),
    Array(Vec<RespValue>),
    Error(String),
    Integer(i64),
    Null,
}

impl RespValue {
    fn serialize(self) -> Vec<u8> {
        match self {
            RespValue::SimpleString(s) => format!("+{}\r\n", s).into_bytes(),
            RespValue::BulkString(s) => format!("${}\r\n{}\r\n", s.len(), s).into_bytes(),
            RespValue::Error(msg) => format!("-ERR {}\r\n", msg).into_bytes(),
            RespValue::Integer(i) => format!(":{}\r\n", i).into_bytes(),
            RespValue::Null => b"$-1\r\n".to_vec(),
            RespValue::Array(elems) => {
                let mut out = format!("*{}\r\n", elems.len()).into_bytes();
                for el in elems {
                    out.extend_from_slice(&el.serialize());
                }
                out
            }
        }
    }
}

fn parse_resp(input: &str) -> Option<RespValue> {
    let mut lines = input.lines();
    let first_line = lines.next()?;

    match &first_line[0..1] {
        "*" => {
            let len = first_line[1..].parse::<usize>().ok()?;
            let mut elements = Vec::with_capacity(len);

            for _ in 0..len {
                let prefix = lines.next()?;
                if prefix.starts_with("$") {
                    let data = lines.next()?;
                    elements.push(RespValue::BulkString(data.to_string()));
                }
            }
            Some(RespValue::Array(elements))
        }
        "+" => Some(RespValue::SimpleString(first_line[1..].to_string())),
        "$" => {
            let data = lines.next()?;
            Some(RespValue::BulkString(data.to_string()))
        }
        _ => None,
    }
}

fn execute_command(cmd: Command, db: &Db) -> RespValue {
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

fn extract_string(elems: &[RespValue], index: usize) -> Option<String> {
    match elems.get(index) {
        Some(RespValue::BulkString(s)) => Some(s.clone()),
        _ => None,
    }
}
fn main() {
    let db: Db = Arc::new(Mutex::new(HashMap::new()));
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let db_clone = db.clone();

                thread::spawn(move || {
                    loop {
                        let mut buffer = [0; 512];
                        let bytes_read = stream.read(&mut buffer);

                        match bytes_read {
                            Ok(0) => break,
                            Ok(n) => {
                                let input = String::from_utf8_lossy(&buffer[..n]);
                                if let Some(resp_data) = parse_resp(&input) {
                                    let response_to_send = match Command::from_resp(resp_data) {
                                        Ok(cmd) => execute_command(cmd, &db_clone),
                                        Err(e) => RespValue::Error(e),
                                    };
                                    let _ = stream.write_all(&response_to_send.serialize());
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

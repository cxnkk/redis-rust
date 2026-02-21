use crate::{resp::RespValue, storage::extract_string};

#[derive(Debug)]
pub enum Command {
    Ping(Option<String>),
    Echo(String),
    Set(String, String, Option<u64>),
    Get(String),
    RPush(String, Vec<String>),
    LRange(String, (usize, usize)),
}

impl Command {
    pub fn from_resp(resp: RespValue) -> Result<Self, String> {
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

                    let mut px = None;
                    if let Some(flag) = extract_string(&elems, 3) {
                        if flag.to_uppercase() == "PX" {
                            let ms_str =
                                extract_string(&elems, 4).ok_or("PX requires milliseconds")?;
                            px = Some(ms_str.parse::<u64>().map_err(|_| "Invalid PX value")?);
                        }
                    }

                    Ok(Self::Set(key, val, px))
                }
                "GET" => match elems.get(1) {
                    Some(RespValue::BulkString(s)) => Ok(Self::Get(s.clone())),
                    _ => Err("GET requires a key".to_string()),
                },
                "RPUSH" => {
                    let key = extract_string(&elems, 1).ok_or("RPUSH missing key")?;

                    let mut values = Vec::new();
                    for i in 2..elems.len() {
                        if let Some(s) = extract_string(&elems, i) {
                            values.push(s);
                        }
                    }

                    if values.is_empty() {
                        return Err("RPUSH requires at least one value".to_string());
                    }

                    Ok(Self::RPush(key, values))
                }
                "LRANGE" => {
                    let key = extract_string(&elems, 1).ok_or("RPUSH missing key")?;
                    let start = extract_string(&elems, 2)
                        .ok_or("RPUSH missing start")?
                        .trim()
                        .parse()
                        .unwrap();
                    let stop = extract_string(&elems, 3)
                        .ok_or("RPUSH missing stop")?
                        .trim()
                        .parse()
                        .unwrap();

                    Ok(Self::LRange(key, (start, stop)))
                }
                _ => Err(format!("Unknown command: {}", cmd_name)),
            }
        } else {
            Err("Expected array of bulk strings".to_string())
        }
    }
}

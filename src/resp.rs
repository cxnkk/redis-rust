#[allow(dead_code)]
#[derive(Debug)]
pub enum RespValue {
    SimpleString(String),
    BulkString(String),
    Array(Vec<RespValue>),
    Error(String),
    Integer(i64),
    Null,
}

impl RespValue {
    pub fn serialize(self) -> Vec<u8> {
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

pub fn parse_resp(input: &str) -> Option<RespValue> {
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

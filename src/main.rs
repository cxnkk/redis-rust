mod command;
mod resp;
mod storage;

use crate::{
    command::Command,
    resp::{RespValue, parse_resp},
    storage::{Db, execute_command},
};

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

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

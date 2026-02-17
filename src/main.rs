#![allow(unused_imports)]
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

fn parse_resp(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut lines = input.lines();

    if let Some(first_line) = lines.next() {
        if first_line.starts_with("*") {
            if let Ok(array_len) = first_line[1..].parse::<usize>() {
                for _ in 0..array_len {
                    if let Some(len_line) = lines.next() {
                        if len_line.starts_with("$") {
                            if let Some(data_line) = lines.next() {
                                result.push(data_line.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    result
}

fn process_command(command: &str) -> Vec<u8> {
    let args: Vec<&str> = command.split_whitespace().collect();

    if args.is_empty() {
        return b"-ERR empty command\r\n".to_vec();
    }

    let cmd = args[0].to_uppercase();

    match cmd.as_str() {
        "PING" => {
            if args.len() > 1 {
                format!("${}\r\n{}\r\n", args[1].len(), args[1]).into_bytes()
            } else {
                b"+PONG\r\n".to_vec()
            }
        }
        "ECHO" => {
            if args.len() > 1 {
                let echo_arg = args[1..].join(" ");
                format!("${}\r\n{}\r\n", echo_arg.len(), echo_arg).into_bytes()
            } else {
                b"-ERR wrong number of arguments for 'echo' command\r\n".to_vec()
            }
        }
        _ => b"-ERR unknown command\r\n".to_vec(),
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                thread::spawn(move || {
                    loop {
                        let mut buffer = [0; 512];
                        let bytes_read = stream.read(&mut buffer);

                        match bytes_read {
                            Ok(0) => break,
                            Ok(n) => {
                                let input = String::from_utf8_lossy(&buffer[..n]);
                                let args = parse_resp(&input);

                                let response = if args.is_empty() {
                                    b"-ERR invalid request\r\n".to_vec()
                                } else {
                                    let command = args.join(" ");
                                    process_command(&command)
                                };

                                let _ = stream.write_all(&response);
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

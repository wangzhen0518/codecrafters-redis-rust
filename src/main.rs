#![allow(unused_imports)]
use std::{
    cmp::min,
    collections::HashMap,
    io::{Read, Write},
    iter,
    net::{TcpListener, TcpStream},
};

fn read_number(input: &[u8], start_index: usize) -> (usize, usize) {
    let mut index = start_index;
    while index < input.len() && input[index] != b'\r' {
        index += 1;
    }
    let mut number = 0;
    for c in &input[start_index..index] {
        number = number * 10 + (c - b'0') as usize;
    }
    (number, index + 2 - start_index)
}

fn read_head(input: &[u8], start_index: usize) -> (usize, usize) {
    assert_eq!(input[start_index], b'*');
    let (part_num, index_offset) = read_number(input, start_index + 1);
    (part_num, index_offset + 1)
}

fn read_item(input: &[u8], start_index: usize) -> (&[u8], usize) {
    assert_eq!(input[start_index], b'$');
    let (byte_len, index_offset) = read_number(input, start_index + 1);
    let byte_start = start_index + 1 + index_offset;
    let byte_end = byte_start + byte_len;
    (&input[byte_start..byte_end], byte_end + 2 - start_index)
}

fn read_one_command(input: &[u8], start_index: usize) -> (Vec<&[u8]>, usize) {
    let mut current_command = vec![];
    let mut index = start_index;

    let (part_num, index_offset) = read_head(input, index);
    index += index_offset;

    for _ in 0..part_num {
        let (item, index_offset) = read_item(input, index);
        current_command.push(item);
        index += index_offset;
    }

    (current_command, index - start_index)
}

fn parse_input(input: &[u8]) -> Vec<Vec<&[u8]>> {
    let mut command_list = vec![];
    let mut index = 0;
    while index < input.len() {
        let (current_command, index_offset) = read_one_command(input, index);
        command_list.push(current_command);
        index += index_offset;
    }
    command_list
}

fn read_all(stream: &mut TcpStream) -> Vec<u8> {
    let mut full_buffer = vec![];
    let mut buffer = [0u8; 1024];
    let mut num = stream.read(&mut buffer).unwrap();
    while num == buffer.len() {
        full_buffer.extend_from_slice(&buffer[..num]);
        num = stream.read(&mut buffer).unwrap();
    }
    full_buffer.extend_from_slice(&buffer[..num]);
    full_buffer
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    let mut database: HashMap<&str, &[u8]> = HashMap::new();
    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                // _stream.write_all(b"+PONG\r\n").unwrap();
                let mut input = read_all(&mut _stream);
                while !input.is_empty() {
                    println!("{}", "=".repeat(50));
                    let input_str = str::from_utf8(&input).expect("Invalid UTF-8");
                    println!("{}", input_str);

                    let char_list: Vec<char> = input.iter().map(|c| char::from(*c)).collect();
                    println!("{:?}", char_list);
                    println!("{}", "=".repeat(50));

                    let command_list = parse_input(&input);
                    for command_args in command_list {
                        let command = str::from_utf8(command_args[0])
                            .expect("Invalid UTF-8")
                            .to_lowercase();
                        match command.as_str() {
                            "ping" => _stream
                                .write_all(b"+PONG\r\n")
                                .expect("Response PONG Failed"),
                            "set" => {}
                            "get" => {}
                            _ => {}
                        }
                    }

                    input = read_all(&mut _stream);
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

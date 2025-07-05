use std::{collections::HashMap, sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

lazy_static! {
    static ref DATABASE: Arc<Mutex<HashMap<String, Vec<u8>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

fn ascii_to_number(ascii_bytes: &[u8]) -> usize {
    let mut number = 0;
    for c in ascii_bytes {
        number = number * 10 + (c - b'0') as usize;
    }
    number
}

fn read_number(input: &[u8], start_index: usize) -> (usize, usize) {
    let mut index = start_index;
    while index < input.len() && input[index] != b'\r' {
        index += 1;
    }
    let number = ascii_to_number(&input[start_index..index]);
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

async fn read_all(stream: &mut TcpStream) -> Vec<u8> {
    let mut full_buffer = vec![];
    let mut buffer = [0u8; 1024];
    let mut num = stream.read(&mut buffer).await.unwrap();
    while num == buffer.len() {
        full_buffer.extend_from_slice(&buffer[..num]);
        num = stream.read(&mut buffer).await.unwrap();
    }
    full_buffer.extend_from_slice(&buffer[..num]);
    full_buffer
}

fn encode_response_string(response: Option<&[u8]>) -> String {
    if let Some(response) = response {
        format!(
            "${}\r\n{}\r\n",
            response.len(),
            str::from_utf8(response).expect("Invalid UTF-8")
        )
    } else {
        "$-1\r\n".to_string()
    }
}

fn encode_status_string(status: &str) -> String {
    format!("+{}\r\n", status)
}

fn expire_key(key: String, time: u64) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(time)).await;
        DATABASE.lock().await.remove(&key);
    });
}

async fn execute_one_command(command_args: Vec<&[u8]>, stream: &mut TcpStream) {
    let command = str::from_utf8(command_args[0])
        .expect("Invalid UTF-8")
        .to_lowercase();
    match command.as_str() {
        "ping" => stream
            .write_all(encode_status_string("PONG").as_bytes())
            .await
            .expect("Response PONG Failed"),
        "echo" => {
            let response = command_args[1];
            let encode_resp = encode_response_string(Some(response));
            stream
                .write_all(encode_resp.as_bytes())
                .await
                .expect("Response echo failed");
        }
        "set" => {
            let key = str::from_utf8(command_args[1])
                .expect("Invalid UTF-8")
                .to_string();
            let value = command_args[2].to_vec();
            DATABASE.lock().await.insert(key.clone(), value);

            if command_args.len() >= 5 {
                let ex_command = str::from_utf8(command_args[3])
                    .expect("Invalid UTF-8")
                    .to_lowercase();
                if &ex_command == "px" {
                    let time = ascii_to_number(command_args[4]);
                    expire_key(key.clone(), time as u64);
                }
            }

            stream
                .write_all(encode_status_string("OK").as_bytes())
                .await
                .expect("Failed to respond to SET");
        }
        "get" => {
            let key = str::from_utf8(command_args[1]).expect("Invalid UTF-8");
            let database = DATABASE.lock().await;
            let value = database.get(key).map(|value| value.as_slice());
            stream
                .write_all(encode_response_string(value).as_bytes())
                .await
                .expect("Response get failed");
        }
        _ => {}
    }
}

async fn handle_connection(mut stream: TcpStream) {
    println!("accepted new connection");
    tokio::spawn(async move {
        let mut input = read_all(&mut stream).await;
        while !input.is_empty() {
            println!("{}", "=".repeat(50));
            let input_str = str::from_utf8(&input).expect("Invalid UTF-8");
            println!("{}", input_str);

            let char_list: Vec<char> = input.iter().map(|c| char::from(*c)).collect();
            println!("{:?}", char_list);
            println!("{}", "=".repeat(50));

            let command_list = parse_input(&input);
            for command_args in command_list {
                execute_one_command(command_args, &mut stream).await;
            }

            input = read_all(&mut stream).await;
        }
    });
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .expect("Failed to bind port 6379");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => handle_connection(stream).await,
            Err(e) => println!("error: {}", e),
        }
    }
}

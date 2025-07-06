use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

#[derive(Debug)]
pub(crate) struct Info {
    lib_name: String,
    lib_ver: String,
}

#[derive(Debug)]
pub(crate) struct State {
    id: usize,
    info: Info,
    db: HashMap<String, Vec<u8>>,
}

#[derive(Debug)]
pub(crate) struct Handler {
    state: Mutex<State>,
}

impl Handler {
    pub fn new(lib_name: String, lib_ver: String) -> Arc<Handler> {
        Arc::new(Handler {
            state: Mutex::new(State {
                id: 0,
                info: Info { lib_name, lib_ver },
                db: HashMap::new(),
            }),
        })
    }

    fn expire_key(self: &Arc<Self>, key: String, time: u64) {
        let handler = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(time)).await;
            handler.state.lock().await.db.remove(&key);
        });
    }

    async fn execute_one_command(
        self: &Arc<Handler>,
        command_args: Vec<&[u8]>,
        stream: &mut TcpStream,
        addr: &SocketAddr,
    ) {
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
                    .write_all(&encode_resp)
                    .await
                    .expect("Response echo failed");
            }
            "set" => {
                let key = str::from_utf8(command_args[1])
                    .expect("Invalid UTF-8")
                    .to_string();
                let value = command_args[2].to_vec();
                self.state.lock().await.db.insert(key.clone(), value);

                if command_args.len() >= 5 {
                    let ex_command = str::from_utf8(command_args[3])
                        .expect("Invalid UTF-8")
                        .to_lowercase();
                    if &ex_command == "px" {
                        let time = ascii_to_number(command_args[4]);
                        self.expire_key(key.clone(), time as u64);
                    }
                }

                stream
                    .write_all(encode_status_string("OK").as_bytes())
                    .await
                    .expect("Failed to respond to SET");
            }
            "get" => {
                let key = str::from_utf8(command_args[1]).expect("Invalid UTF-8");
                let database = &mut self.state.lock().await.db;
                let value = database.get(key).map(|value| value.as_slice());
                stream
                    .write_all(&encode_response_string(value))
                    .await
                    .expect("Response get failed");
            }
            "client" => {
                let subcommand = str::from_utf8(command_args[1])
                    .expect("Invalid UTF-8")
                    .to_lowercase();
                let resp = match subcommand.as_str() {
                    "info" => {
                        let state = self.state.lock().await;
                        let info_content = format!(
                            "id={} addr={}:{} laddr=127.0.0.1:6379 \
fd=24 name= age=0 idle=0 flags=N db=0 sub=0 psub=0 ssub=0 multi=-1 \
watch=0 qbuf=26 qbuf-free=20448 argv-mem=10 multi-mem=0 rbs=16384 \
rbp=16384 obl=0 oll=0 omem=0 tot-mem=37786 events=r \
cmd=client|info user=default redir=-1 resp=2 \
lib-name={} lib-ver={} io-thread=0\n",
                            state.id,
                            addr.ip(),
                            addr.port(),
                            state.info.lib_name,
                            state.info.lib_ver
                        );
                        encode_response_string(Some(info_content.as_bytes()))
                    }
                    "setinfo" => {
                        let mut index = 2;
                        while index < command_args.len() {
                            let attr_name = str::from_utf8(command_args[index])
                                .expect("Invalid UTF-8")
                                .to_lowercase();
                            let attr_value = str::from_utf8(command_args[index + 1])
                                .expect("Invalid UTF-8")
                                .to_string();
                            let mut state = self.state.lock().await;
                            match attr_name.as_str() {
                                "lib-name" => state.info.lib_name = attr_value,
                                "lib-ver" => state.info.lib_ver = attr_value,
                                _ => {}
                            }
                            index += 2;
                        }

                        encode_status_string("OK").as_bytes().to_vec()
                    }
                    _ => {
                        unimplemented!()
                    }
                };
                stream.write_all(&resp).await.expect("Response get failed");
            }
            _ => {}
        }
    }

    pub async fn handle_connection(self: &Arc<Handler>, mut stream: TcpStream, addr: SocketAddr) {
        let handler = self.clone();
        println!("accepted new connection");
        tokio::spawn(async move {
            handler.state.lock().await.id += 1;
            let mut input = read_all(&mut stream).await;
            while !input.is_empty() {
                println!("{}", "=".repeat(50));

                if let Ok(input_str) = str::from_utf8(&input) {
                    println!("{}", input_str);
                }

                let char_list: Vec<char> = input.iter().map(|c| char::from(*c)).collect();
                println!("{:?}", char_list);
                println!("{}", "=".repeat(50));

                // let command_list = parse_input(&input);
                // for command_args in command_list {
                //     execute_one_command(command_args, &mut stream).await;
                // }

                let (command_args, _) = read_one_command(&input, 0);
                handler
                    .execute_one_command(command_args, &mut stream, &addr)
                    .await;

                input = read_all(&mut stream).await;
            }
        });
    }
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

// fn parse_input(input: &[u8]) -> Vec<Vec<&[u8]>> {
//     let mut command_list = vec![];
//     let mut index = 0;
//     while index < input.len() {
//         let (current_command, index_offset) = read_one_command(input, index);
//         command_list.push(current_command);
//         index += index_offset;
//     }
//     command_list
// }

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

fn encode_response_string(response: Option<&[u8]>) -> Vec<u8> {
    if let Some(response) = response {
        [
            b"$",
            response.len().to_string().as_bytes(),
            b"\r\n",
            response,
            b"\r\n",
        ]
        .concat()
    } else {
        b"$-1\r\n".to_vec()
    }
}

fn encode_status_string(status: &str) -> String {
    format!("+{}\r\n", status)
}

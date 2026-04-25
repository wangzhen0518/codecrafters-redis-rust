use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use bytes::{Bytes, BytesMut};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
    time::Instant,
};

use crate::{
    command::{self, ExecuteCommand, parse_command},
    resp::{self, RespData, parse_client_request, serialize_resp, serialize_simple_error},
};

const BUFFER_INITIAL_SIZE: usize = 0;

pub type Key = Bytes;
pub type Value = Bytes;
pub type DbItem = (Value, Option<Instant>);
pub type Db = HashMap<Key, DbItem>;

pub struct Server {
    pub addr: SocketAddr,
    pub rdb_filename: PathBuf,
    pub db: Db,
    pub conn_num: u64,
}

impl Server {
    pub fn new(addr: SocketAddr, rdb_filename: PathBuf) -> Self {
        Self {
            addr,
            rdb_filename,
            db: HashMap::new(),
            conn_num: 0,
        }
    }
}

pub struct Connection {
    pub id: u64,
    pub addr: SocketAddr,
    pub stream: TcpStream,
    pub name: String,
    pub lib_name: String,
    pub lib_ver: String,
}

impl Connection {
    pub fn new(id: u64, addr: SocketAddr, stream: TcpStream) -> Self {
        Self {
            id,
            addr,
            stream,
            name: String::new(),
            lib_name: String::new(),
            lib_ver: String::new(),
        }
    }
}

#[derive(Debug, Error)]
enum Error {
    #[error("Parse RESP failed: {}", .0)]
    RespParseError(#[from] resp::ParseError),

    #[error("Parse redis command failed: {}", .0)]
    CommandParseError(#[from] command::ParseError),

    #[error("Execute command failed: {}", .0)]
    CommandExecError(#[from] command::ExecError),
}

async fn read_all(stream: &mut TcpStream, buffer: &mut BytesMut) -> usize {
    //todo 考虑如何支持流式 parse，不然无法确定 stream 是不是已经收到了 client 发送的全部数据
    stream.read_buf(buffer).await.unwrap_or_default()
    // .expect("Failed to read from tcp stream")
}

pub async fn handle_connection(server: Arc<Mutex<Server>>, mut conn: Connection) {
    let mut input_buffer = BytesMut::with_capacity(BUFFER_INITIAL_SIZE);
    let mut output_buffer = BytesMut::with_capacity(BUFFER_INITIAL_SIZE);

    let mut n = read_all(&mut conn.stream, &mut input_buffer).await;

    while !input_buffer.is_empty() {
        tracing::debug!("{}", "=".repeat(50));

        if let Ok(input_str) = str::from_utf8(&input_buffer) {
            tracing::debug!("{:?}", input_str);
        } else {
            let char_list: Vec<char> = input_buffer.iter().map(|c| char::from(*c)).collect();
            tracing::debug!("{:?}", char_list);
        }
        tracing::debug!("{}", "=".repeat(50));

        let result: Result<RespData, Error> = async {
            let request = parse_client_request(&mut input_buffer)?;
            let command = parse_command(&request)?;
            let resp = command.execute(server.clone(), &mut conn).await?;
            Ok(resp)
        }
        .await;

        match result {
            Ok(resp) => serialize_resp(&mut output_buffer, &resp),
            Err(err) => serialize_simple_error(&mut output_buffer, err.to_string().as_str()),
        }

        conn.stream.write_all_buf(&mut output_buffer).await.ok();
        // .expect("Failed to write response to tcp stream");
        input_buffer.clear();
        n = read_all(&mut conn.stream, &mut input_buffer).await;
    }
    server.lock().await.conn_num -= 1;
}

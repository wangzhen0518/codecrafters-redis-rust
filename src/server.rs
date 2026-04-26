use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use bytes::{Buf, Bytes, BytesMut};
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
    utils::BytesInStr,
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
    stream.read_buf(buffer).await.unwrap_or_default()
}

pub async fn handle_connection(server: Arc<Mutex<Server>>, mut conn: Connection) {
    let mut input_buffer = BytesMut::with_capacity(BUFFER_INITIAL_SIZE);
    let mut output_buffer = BytesMut::with_capacity(BUFFER_INITIAL_SIZE);

    while let Ok(n) = conn.stream.read_buf(&mut input_buffer).await
        && n > 0
    {
        tracing::info!("Command: {}", BytesInStr::from_bytes(&input_buffer));

        // Stream-friendly parse: parse on a snapshot and consume input only after a full frame.
        let mut parsing_buffer = input_buffer.clone();
        let result: Result<RespData, Error> = async {
            let request = parse_client_request(&mut parsing_buffer)?;
            let command = parse_command(&request)?;
            let resp = command.execute(server.clone(), &mut conn).await?;
            Ok(resp)
        }
        .await;

        match result {
            Err(Error::RespParseError(resp::ParseError::Eof(_))) => continue,
            Ok(resp) => serialize_resp(&mut output_buffer, &resp),
            Err(err) => serialize_simple_error(&mut output_buffer, err.to_string().as_str()),
        }

        let consumed = input_buffer.len() - parsing_buffer.len();
        input_buffer.advance(consumed);
        if let Err(err) = conn.stream.write_all_buf(&mut output_buffer).await {
            tracing::error!("Failed to send result to client: {}", err);
        }
    }

    server.lock().await.conn_num -= 1;
}

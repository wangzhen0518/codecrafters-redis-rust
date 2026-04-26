use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::Mutex;

use crate::{
    command::{
        client::Client,
        config::Config,
        echo::Echo,
        error::{ExecResult, ParseResult},
        get::Get,
        ping::Ping,
        set::Set,
        unknown::Unknown,
    },
    resp::{ClientRequest, RespData},
    server::{Connection, Server},
    utils::BytesInStr,
};

mod client;
mod config;
mod echo;
mod error;
mod get;
mod ping;
mod set;
mod unknown;

pub use error::{ExecError, ParseError};

#[derive(Debug, PartialEq)]
pub enum Command {
    Ping(Ping),
    Echo(Echo),
    Get(Get),
    Set(Set),
    Client(Client),
    Config(Config),
    Unknown(Unknown),
}

// ======================================== Parse ========================================
trait Parse {
    fn parse(args: &[Bytes]) -> ParseResult<Self>
    where
        Self: std::marker::Sized;
}

#[inline]
fn check_length_eq(args: &[Bytes], length: usize) -> ParseResult<()> {
    if args.len() != length {
        return Err(ParseError::ExpectLengthEq(
            length,
            args.len(),
            args.to_vec(),
        ));
    }
    Ok(())
}

#[inline]
fn check_length_ge(args: &[Bytes], length: usize) -> ParseResult<()> {
    if args.len() < length {
        return Err(ParseError::ExpectLengthGe(
            length,
            args.len(),
            args.to_vec(),
        ));
    }
    Ok(())
}

pub fn parse_command(request: &ClientRequest) -> ParseResult<Command> {
    let command = match request.command.to_uppercase().as_str() {
        "PING" => Command::Ping(Ping::parse(&request.args)?),
        "ECHO" => Command::Echo(Echo::parse(&request.args)?),
        "GET" => Command::Get(Get::parse(&request.args)?),
        "SET" => Command::Set(Set::parse(&request.args)?),
        "CLIENT" => Command::Client(Client::parse(&request.args)?),
        "CONFIG" => Command::Config(Config::parse(&request.args)?),
        command => {
            tracing::debug!(
                "Unknown command: `{}`, args: `{:?}`",
                &command,
                BytesInStr::from_bytes_array(&request.args)
            );
            Command::Unknown(Unknown::parse(&request.args)?)
        }
    };
    Ok(command)
}

// ======================================== Execute ========================================
pub trait ExecuteCommand {
    async fn execute(
        &self,
        server: Arc<Mutex<Server>>,
        conn: &mut Connection,
    ) -> ExecResult<RespData>;
}

impl ExecuteCommand for Command {
    async fn execute(
        &self,
        server: Arc<Mutex<Server>>,
        conn: &mut Connection,
    ) -> ExecResult<RespData> {
        match self {
            Command::Ping(ping) => ping.execute(server, conn).await,
            Command::Echo(echo) => echo.execute(server, conn).await,
            Command::Get(get) => get.execute(server, conn).await,
            Command::Set(set) => set.execute(server, conn).await,
            Command::Client(client) => client.execute(server, conn).await,
            Command::Config(config) => config.execute(server, conn).await,
            Command::Unknown(unknown) => unknown.execute(server, conn).await,
        }
    }
}

#[cfg(test)]
pub(super) mod test {
    use std::{path::PathBuf, sync::Arc};

    use bytes::Bytes;
    use tokio::{
        net::{TcpListener, TcpStream},
        sync::Mutex,
    };

    use crate::{
        resp::ClientRequest,
        server::{Connection, Server},
    };

    pub fn build_request(command: &str, args: &[&str]) -> ClientRequest {
        ClientRequest {
            command: command.to_string(),
            args: args
                .iter()
                .map(|arg| Bytes::copy_from_slice(arg.as_bytes()))
                .collect(),
        }
    }

    pub async fn build_server_connection() -> (Arc<Mutex<Server>>, Connection) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local listener");
        let server_addr = listener.local_addr().expect("listener address");
        let connect = tokio::spawn(async move {
            TcpStream::connect(server_addr)
                .await
                .expect("connect to local listener")
        });
        let (server_stream, client_addr) =
            listener.accept().await.expect("accept local connection");
        let _client_stream = connect.await.expect("join connect task");

        (
            Arc::new(Mutex::new(Server::new(
                server_addr,
                PathBuf::from("/tmp/dump.rdb"),
            ))),
            Connection::new(1, client_addr, server_stream),
        )
    }
}

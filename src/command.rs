use std::sync::Arc;

use bytes::{Buf, Bytes, BytesMut};
use tokio::sync::Mutex;

use crate::{
    command::{
        client::Client,
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
mod echo;
mod error;
mod get;
mod ping;
mod set;
mod unknown;

pub use error::{ExecError, ParseError};

pub enum Command {
    Ping(Ping),
    Echo(Echo),
    Get(Get),
    Set(Set),
    Client(Client),
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
        return Err(ParseError::ExpectLengthEq(0, args.len(), args.to_vec()));
    }
    Ok(())
}

#[inline]
fn check_length_ge(args: &[Bytes], length: usize) -> ParseResult<()> {
    if args.len() < length {
        return Err(ParseError::ExpectLengthGe(0, args.len(), args.to_vec()));
    }
    Ok(())
}

pub fn parse_command(request: &ClientRequest) -> ParseResult<Command> {
    let command = match request.command.as_str() {
        "PING" => Command::Ping(Ping::parse(&request.args)?),
        "ECHO" => Command::Echo(Echo::parse(&request.args)?),
        "GET" => Command::Get(Get::parse(&request.args)?),
        "SET" => Command::Set(Set::parse(&request.args)?),
        "CLIENT" => Command::Client(Client::parse(&request.args)?),
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
            Command::Unknown(unknown) => unknown.execute(server, conn).await,
        }
    }
}

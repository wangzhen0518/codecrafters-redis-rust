use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::Mutex;

use crate::{
    command::{ExecuteCommand, Parse, ParseResult, check_length_eq, error::ExecResult},
    resp::RespData,
    server::{Connection, Server},
};

#[derive(Debug, PartialEq)]
pub struct Ping;

impl Parse for Ping {
    fn parse(args: &[Bytes]) -> ParseResult<Self>
    where
        Self: std::marker::Sized,
    {
        check_length_eq(args, 0)?;
        Ok(Ping)
    }
}

impl ExecuteCommand for Ping {
    async fn execute(
        &self,
        _server: Arc<Mutex<Server>>,
        _conn: &mut Connection,
    ) -> ExecResult<RespData> {
        Ok(RespData::SimpleString("PONG".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::Ping;
    use crate::{
        command::{
            Command, ExecuteCommand, ParseError, parse_command,
            test::{build_request, build_server_connection},
        },
        resp::RespData,
    };

    #[test]
    fn parse_ping_should_accept_no_arguments() {
        let cmd = parse_command(&build_request("PING", &[])).expect("parse ping");
        assert_eq!(cmd, Command::Ping(Ping));
    }

    #[test]
    fn parse_ping_should_reject_extra_arguments() {
        let err = parse_command(&build_request("PING", &["x"]))
            .expect_err("ping with args should fail");
        assert_eq!(
            err,
            ParseError::ExpectLengthEq(0, 1, vec![Bytes::from_owner("x")])
        );
    }

    #[tokio::test]
    async fn execute_ping_should_return_pong() {
        let (server, mut conn) = build_server_connection().await;
        let cmd = Ping;
        let resp = cmd.execute(server, &mut conn).await.expect("execute ping");
        assert_eq!(resp, RespData::SimpleString("PONG".to_string()));
    }
}

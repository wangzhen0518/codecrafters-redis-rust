use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::Mutex;

use crate::{
    command::{ExecuteCommand, Parse, ParseResult, check_length_eq, error::ExecResult},
    resp::RespData,
    server::{Connection, Server},
};

#[derive(Debug, PartialEq)]
pub struct Echo {
    content: Bytes,
}

impl Parse for Echo {
    fn parse(args: &[Bytes]) -> ParseResult<Self>
    where
        Self: std::marker::Sized,
    {
        check_length_eq(args, 1)?;
        Ok(Echo {
            content: args[0].clone(),
        })
    }
}

impl ExecuteCommand for Echo {
    async fn execute(
        &self,
        _server: Arc<Mutex<Server>>,
        _conn: &mut Connection,
    ) -> ExecResult<RespData> {
        Ok(RespData::BulkString(Some(self.content.clone())))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::Echo;
    use crate::{
        command::{
            Command, ExecuteCommand, ParseError, parse_command,
            test::{build_request, build_server_connection},
        },
        resp::RespData,
    };

    #[test]
    fn parse_echo_should_read_single_argument() {
        let cmd = parse_command(&build_request("ECHO", &["hello"])).expect("parse echo");
        assert_eq!(
            cmd,
            Command::Echo(Echo {
                content: Bytes::from_owner("hello")
            })
        );
    }

    #[test]
    fn parse_echo_should_reject_missing_argument() {
        let err = parse_command(&build_request("ECHO", &[])).expect_err("echo requires one arg");
        assert_eq!(err, ParseError::ExpectLengthEq(1, 0, vec![]));
    }

    #[test]
    fn parse_echo_should_reject_extra_argument() {
        let err = parse_command(&build_request("ECHO", &["a", ""]))
            .expect_err("echo should not accept two args");
        assert_eq!(
            err,
            ParseError::ExpectLengthEq(1, 2, vec![Bytes::from_owner("a"), Bytes::from_owner("")])
        );
    }

    #[tokio::test]
    async fn execute_echo_should_return_input() {
        let (server, mut conn) = build_server_connection().await;
        let cmd = Echo {
            content: Bytes::from_owner("hello"),
        };
        let resp = cmd.execute(server, &mut conn).await.expect("execute echo");
        assert_eq!(resp, RespData::BulkString(Some(Bytes::from_owner("hello"))));
    }
}

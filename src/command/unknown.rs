use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::Mutex;

use crate::{
    command::{ExecuteCommand, Parse, ParseResult, error::ExecResult},
    resp::RespData,
    server::{Connection, Server},
};

#[derive(Debug, PartialEq)]
pub struct Unknown {
    args: Vec<Bytes>,
}

impl Parse for Unknown {
    fn parse(args: &[Bytes]) -> ParseResult<Self>
    where
        Self: std::marker::Sized,
    {
        Ok(Unknown {
            args: args.to_vec(),
        })
    }
}

impl ExecuteCommand for Unknown {
    async fn execute(
        &self,
        _server: Arc<Mutex<Server>>,
        _conn: &mut Connection,
    ) -> ExecResult<RespData> {
        Ok(RespData::SimpleString(format!(
            "Unknown command with arguments: {:?}",
            self.args
        )))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::Unknown;
    use crate::{
        command::{
            Command, ExecuteCommand, parse_command,
            test::{build_request, build_server_connection},
        },
        resp::RespData,
    };

    #[test]
    fn parse_unknown_should_keep_all_arguments() {
        let cmd = parse_command(&build_request("MYSTERY", &["a", "b"]))
            .expect("parse unknown");
        assert_eq!(
            cmd,
            Command::Unknown(Unknown {
                args: vec![Bytes::copy_from_slice(b"a"), Bytes::copy_from_slice(b"b")]
            })
        );
    }

    #[test]
    fn parse_unknown_should_allow_empty_arguments() {
        let cmd = parse_command(&build_request("MYSTERY", &[])).expect("parse unknown");
        assert_eq!(cmd, Command::Unknown(Unknown { args: vec![] }));
    }

    #[tokio::test]
    async fn execute_unknown_should_return_message() {
        let (server, mut conn) = build_server_connection().await;
        let cmd = Unknown {
            args: vec![Bytes::copy_from_slice(b"a"), Bytes::copy_from_slice(b"b")],
        };
        let resp = cmd
            .execute(server, &mut conn)
            .await
            .expect("execute unknown");
        assert_eq!(
            resp,
            RespData::SimpleString(
                "Unknown command with arguments: [b\"a\", b\"b\"]".to_string()
            )
        );
    }

    #[tokio::test]
    async fn execute_unknown_should_return_message_for_empty_args() {
        let (server, mut conn) = build_server_connection().await;
        let cmd = Unknown { args: vec![] };
        let resp = cmd
            .execute(server, &mut conn)
            .await
            .expect("execute unknown");
        assert_eq!(
            resp,
            RespData::SimpleString("Unknown command with arguments: []".to_string())
        );
    }
}

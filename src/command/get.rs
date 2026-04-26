use std::sync::Arc;

use bytes::Bytes;
use tokio::{sync::Mutex, time::Instant};

use crate::{
    command::{ExecuteCommand, Parse, ParseResult, check_length_eq, error::ExecResult},
    resp::RespData,
    server::{Connection, Server},
    utils::BytesInStr,
};

#[derive(Debug, PartialEq)]
pub struct Get {
    key: Bytes,
}

impl Parse for Get {
    fn parse(args: &[Bytes]) -> ParseResult<Self>
    where
        Self: std::marker::Sized,
    {
        check_length_eq(args, 1)?;
        Ok(Get {
            key: args[0].clone(),
        })
    }
}

impl ExecuteCommand for Get {
    async fn execute(
        &self,
        server: Arc<Mutex<Server>>,
        _conn: &mut Connection,
    ) -> ExecResult<RespData> {
        let db = &mut server.lock().await.db;

        let Some((value, expire_time)) = db.get(&self.key) else {
            return Ok(RespData::BulkString(None));
        };

        if expire_time.is_some_and(|t| t <= Instant::now()) {
            // If key does not exist, the function has returned in the previous else
            let (value, _) = db.remove(&self.key).unwrap();
            tracing::info!(
                "Remove Key: {}, Value: {}",
                BytesInStr::from_bytes(&self.key),
                BytesInStr::from_bytes(&value)
            );
            return Ok(RespData::BulkString(None));
        }

        Ok(RespData::BulkString(Some(value.clone())))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bytes::Bytes;
    use tokio::time::Instant;

    use super::Get;
    use crate::{
        command::{
            Command, ExecuteCommand, ParseError, parse_command,
            test::{build_request, build_server_connection},
        },
        resp::RespData,
    };

    #[test]
    fn parse_get_should_read_key() {
        let cmd = parse_command(&build_request("GET", &["my-key"])).expect("parse get");
        assert_eq!(
            cmd,
            Command::Get(Get {
                key: Bytes::from_owner("my-key")
            })
        );
    }

    #[test]
    fn parse_get_should_reject_missing_key() {
        let err = parse_command(&build_request("GET", &[])).expect_err("get requires one arg");
        assert_eq!(err, ParseError::ExpectLengthEq(1, 0, vec![]));
    }

    #[test]
    fn parse_get_should_reject_extra_argument() {
        let err = parse_command(&build_request("GET", &["a", ""]))
            .expect_err("get should reject extra arg");
        assert_eq!(
            err,
            ParseError::ExpectLengthEq(1, 2, vec![Bytes::from_owner("a"), Bytes::from_owner("")])
        );
    }

    #[tokio::test]
    async fn execute_get_should_return_existing_value() {
        let (server, mut conn) = build_server_connection().await;
        server.lock().await.db.insert(
            Bytes::from_owner("my-key"),
            (Bytes::from_owner("my-value"), None),
        );

        let cmd = Get {
            key: Bytes::from_owner("my-key"),
        };
        let resp = cmd.execute(server, &mut conn).await.expect("execute get");
        assert_eq!(
            resp,
            RespData::BulkString(Some(Bytes::from_owner("my-value")))
        );
    }

    #[tokio::test]
    async fn execute_get_should_return_null_for_missing_key() {
        let (server, mut conn) = build_server_connection().await;
        let cmd = Get {
            key: Bytes::from_owner("missing"),
        };
        let resp = cmd.execute(server, &mut conn).await.expect("execute get");
        assert_eq!(resp, RespData::BulkString(None));
    }

    #[tokio::test]
    async fn execute_get_should_return_null_and_remove_expired_key() {
        let (server, mut conn) = build_server_connection().await;
        server.lock().await.db.insert(
            Bytes::from_owner("my-key"),
            (
                Bytes::from_owner("my-value"),
                Some(Instant::now() - Duration::from_millis(1)),
            ),
        );

        let cmd = Get {
            key: Bytes::from_owner("my-key"),
        };
        let resp = cmd
            .execute(server.clone(), &mut conn)
            .await
            .expect("execute get");
        assert_eq!(resp, RespData::BulkString(None));
        assert!(
            !server
                .lock()
                .await
                .db
                .contains_key(&Bytes::from_owner("my-key"))
        );
    }
}

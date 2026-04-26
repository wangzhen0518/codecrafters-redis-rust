use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use tokio::{sync::Mutex, time::Instant};

use crate::{
    command::{
        ExecuteCommand, Parse, ParseResult, check_length_ge,
        error::{ExecResult, ParseError},
    },
    resp::RespData,
    server::{Connection, Server},
    utils::BytesInStr,
};

#[derive(Debug, PartialEq)]
pub struct Set {
    key: Bytes,
    value: Bytes,
    /// represent all of the following parameters in milliseconds
    /// EX seconds -- Set the specified expire time, in seconds (a positive integer).
    /// PX milliseconds -- Set the specified expire time, in milliseconds (a positive integer).
    /// EXAT timestamp-seconds -- Set the specified Unix time at which the key will expire, in seconds (a positive integer).
    /// PXAT timestamp-milliseconds -- Set the specified Unix time at which the key will expire, in milliseconds (a positive integer).
    expire_time: Option<u64>,
}
impl Parse for Set {
    fn parse(args: &[Bytes]) -> ParseResult<Self> {
        fn parse_i_to_u64(args: &[Bytes], i: usize) -> ParseResult<u64> {
            let expire_time = lexical_core::parse(
                args.get(i + 1)
                    .ok_or(ParseError::ExpectLengthGe(i + 1, i, args.to_vec()))?,
            )?;
            Ok(expire_time)
        }

        fn check_expire_time_is_none(
            expire_time: &Option<u64>,
            new_name: String,
            new_value: String,
        ) -> ParseResult<()> {
            if let Some(expire_time) = expire_time {
                return Err(ParseError::ValueHasBeenSet {
                    name: "Expire Time",
                    value: expire_time.to_string(),
                    new_name,
                    new_value,
                });
            }
            Ok(())
        }

        check_length_ge(args, 2)?;

        let key = args[0].clone();
        let value = args[1].clone();

        let mut expire_time: Option<u64> = None;

        // 解析可选参数
        let mut i = 2;

        while i < args.len() {
            let argument = str::from_utf8(&args[i])?.to_string();
            match argument.to_uppercase().as_str() {
                "EX" | "EXAT" => {
                    let seconds = parse_i_to_u64(args, i)?;
                    check_expire_time_is_none(&expire_time, argument, seconds.to_string())?;
                    expire_time = Some(seconds * 1000);
                    i += 2;
                }
                "PX" | "PXAT" => {
                    let ms = parse_i_to_u64(args, i)?;
                    check_expire_time_is_none(&expire_time, argument, ms.to_string())?;
                    expire_time = Some(ms);
                    i += 2;
                }
                _ => {
                    return Err(ParseError::InvalidArgument(argument));
                }
            }
        }

        Ok(Set {
            key,
            value,
            expire_time,
        })
    }
}

impl ExecuteCommand for Set {
    async fn execute(
        &self,
        server: Arc<Mutex<Server>>,
        _conn: &mut Connection,
    ) -> ExecResult<RespData> {
        let expire_time = self
            .expire_time
            .map(|ms| Instant::now() + Duration::from_millis(ms));
        server
            .lock()
            .await
            .db
            .insert(self.key.clone(), (self.value.clone(), expire_time));
        tracing::info!(
            "Add Key: {}, Value: {}",
            BytesInStr::from_bytes(&self.key),
            BytesInStr::from_bytes(&self.value)
        );
        Ok(RespData::SimpleString("OK".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use tokio::time::Instant;

    use super::Set;
    use crate::{
        command::{
            Command, ExecuteCommand, ParseError, parse_command,
            test::{build_request, build_server_connection},
        },
        resp::RespData,
    };

    #[test]
    fn parse_set_should_accept_key_value_only() {
        let cmd = parse_command(&build_request("SET", &["k", "v"])).expect("parse set");
        assert_eq!(
            cmd,
            Command::Set(Set {
                key: Bytes::from_owner("k"),
                value: Bytes::from_owner("v"),
                expire_time: None,
            })
        );
    }

    #[test]
    fn parse_set_should_parse_px() {
        let cmd =
            parse_command(&build_request("SET", &["k", "v", "PX", "100"])).expect("parse set px");
        assert_eq!(
            cmd,
            Command::Set(Set {
                key: Bytes::from_owner("k"),
                value: Bytes::from_owner("v"),
                expire_time: Some(100),
            })
        );
    }

    #[test]
    fn parse_set_should_parse_option_case_insensitive() {
        let cmd = parse_command(&build_request("SET", &["k", "v", "px", "7"]))
            .expect("parse set lowercase px");
        assert_eq!(
            cmd,
            Command::Set(Set {
                key: Bytes::from_owner("k"),
                value: Bytes::from_owner("v"),
                expire_time: Some(7),
            })
        );
    }

    #[test]
    fn parse_set_should_parse_ex() {
        let cmd =
            parse_command(&build_request("SET", &["k", "v", "EX", "2"])).expect("parse set ex");
        assert_eq!(
            cmd,
            Command::Set(Set {
                key: Bytes::from_owner("k"),
                value: Bytes::from_owner("v"),
                expire_time: Some(2000),
            })
        );
    }

    #[test]
    fn parse_set_should_parse_pxat() {
        let cmd = parse_command(&build_request("SET", &["k", "v", "PXAT", "123"]))
            .expect("parse set pxat");
        assert_eq!(
            cmd,
            Command::Set(Set {
                key: Bytes::from_owner("k"),
                value: Bytes::from_owner("v"),
                expire_time: Some(123),
            })
        );
    }

    #[test]
    fn parse_set_should_parse_exat() {
        let cmd =
            parse_command(&build_request("SET", &["k", "v", "EXAT", "3"])).expect("parse set exat");
        assert_eq!(
            cmd,
            Command::Set(Set {
                key: Bytes::from_owner("k"),
                value: Bytes::from_owner("v"),
                expire_time: Some(3000),
            })
        );
    }

    #[test]
    fn parse_set_should_reject_too_few_arguments() {
        let err = parse_command(&build_request("SET", &["k"])).expect_err("set requires 2 args");
        assert_eq!(
            err,
            ParseError::ExpectLengthGe(2, 1, vec![Bytes::from_owner("k")])
        );
    }

    #[test]
    fn parse_set_should_reject_invalid_option() {
        let err = parse_command(&build_request("SET", &["k", "v", "NX", "1"]))
            .expect_err("unsupported option");
        assert_eq!(err, ParseError::InvalidArgument("NX".to_string()));
    }

    #[test]
    fn parse_set_should_reject_missing_option_value() {
        let err = parse_command(&build_request("SET", &["k", "v", "PX"]))
            .expect_err("missing expire value");
        assert_eq!(
            err,
            ParseError::ExpectLengthGe(
                3,
                2,
                vec![
                    Bytes::from_owner("k"),
                    Bytes::from_owner("v"),
                    Bytes::from_owner("PX"),
                ]
            )
        );
    }

    #[test]
    fn parse_set_should_reject_multiple_expire_options() {
        let err = parse_command(&build_request("SET", &["k", "v", "PX", "10", "EX", "1"]))
            .expect_err("expire option duplicated");
        assert_eq!(
            err,
            ParseError::ValueHasBeenSet {
                name: "Expire Time",
                value: "10".to_string(),
                new_name: "EX".to_string(),
                new_value: "1".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn execute_set_should_store_value_and_return_ok() {
        let (server, mut conn) = build_server_connection().await;
        let cmd = Set {
            key: Bytes::from_owner("k"),
            value: Bytes::from_owner("v"),
            expire_time: None,
        };

        let resp = cmd
            .execute(server.clone(), &mut conn)
            .await
            .expect("execute set");
        assert_eq!(resp, RespData::SimpleString("OK".to_string()));

        let db = &server.lock().await.db;
        assert_eq!(
            db.get(&Bytes::from_owner("k")),
            Some(&(Bytes::from_owner("v"), None))
        );
    }

    #[tokio::test]
    async fn execute_set_should_store_expire_time_when_given() {
        let (server, mut conn) = build_server_connection().await;
        let cmd = Set {
            key: Bytes::from_owner("k"),
            value: Bytes::from_owner("v"),
            expire_time: Some(1000),
        };

        let set_resp = cmd
            .execute(server.clone(), &mut conn)
            .await
            .expect("execute set");
        assert_eq!(set_resp, RespData::SimpleString("OK".to_string()));

        let expire_at = server
            .lock()
            .await
            .db
            .get(&Bytes::from_owner("k"))
            .and_then(|(_, expire)| *expire);
        assert!(expire_at.is_some_and(|t| t > Instant::now()));
    }
}

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

// todo parse

use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use tokio::{sync::Mutex, time::Instant};

use crate::{
    command::{
        ExecuteCommand, Parse, ParseResult, check_length_eq, check_length_ge,
        error::{ExecResult, ParseError},
    },
    resp::RespData,
    server::{Connection, Server},
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
        check_length_ge(args, 2)?;

        let key = args[0].clone();
        let value = args[1].clone();

        let mut expire_time: Option<u64> = None;

        // 解析可选参数
        let mut i = 2;

        while i < args.len() {
            let argument = str::from_utf8(&args[i])?.to_uppercase();
            match argument.as_str() {
                "EX" | "EXAT" => {
                    let seconds: u64 = lexical_core::parse(
                        &args.get(i + 1).ok_or(ParseError::ExpectLengthGe(
                            i + 1,
                            i,
                            args.to_vec(),
                        ))?,
                    )?;
                    // str::from_utf8(&args.get(i + 1).ok_or(ParseError::ExpectLengthGe(
                    //     i + 1,
                    //     i,
                    //     args.to_vec(),
                    // ))?)?
                    // .parse()?;
                    if expire_time.is_some() {
                        return Err(ParseError::ValueHasBeenSet {
                            name: "Expire Time",
                            value: expire_time.unwrap().to_string(),
                            new_name: argument,
                            new_value: seconds.to_string(),
                        });
                    }
                    expire_time = Some(seconds * 1000);
                    i += 2;
                }
                "PX" | "PXAT" => {
                    let ms: u64 = lexical_core::parse(
                        &args.get(i + 1).ok_or(ParseError::ExpectLengthGe(
                            i + 1,
                            i,
                            args.to_vec(),
                        ))?,
                    )?;
                    // let ms: u64 =
                    //     str::from_utf8(&args.get(i + 1).ok_or(ParseError::ExpectLengthGe(
                    //         i + 1,
                    //         i,
                    //         args.to_vec(),
                    //     ))?)?
                    //     .parse()?;
                    if expire_time.is_some() {
                        return Err(ParseError::ValueHasBeenSet {
                            name: "Expire Time",
                            value: expire_time.unwrap().to_string(),
                            new_name: argument,
                            new_value: ms.to_string(),
                        });
                    }
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
        Ok(RespData::SimpleString("OK".to_string()))
    }
}

// todo parse

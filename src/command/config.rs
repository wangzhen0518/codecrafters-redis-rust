use std::{collections::HashMap, path::Path, sync::Arc};

use bytes::Bytes;
use tokio::sync::Mutex;

use crate::{
    command::{
        ExecuteCommand, Parse, ParseResult, check_length_ge,
        error::{ExecResult, ParseError},
    },
    resp::RespData,
    server::{Connection, Server},
};

pub enum Config {
    Get(Vec<String>),
}

impl Parse for Config {
    fn parse(args: &[Bytes]) -> ParseResult<Self>
    where
        Self: std::marker::Sized,
    {
        check_length_ge(args, 2)?;

        let subcommand = str::from_utf8(&args[0])?.to_uppercase();
        match subcommand.as_str() {
            "GET" => Ok(Config::Get(
                args[1..]
                    .iter()
                    .map(|bytes| str::from_utf8(bytes).map(|s| s.to_uppercase()))
                    .collect::<Result<_, _>>()?,
            )),
            _ => Err(ParseError::InvalidArgument(subcommand)),
        }
    }
}

impl ExecuteCommand for Config {
    async fn execute(
        &self,
        server: Arc<Mutex<Server>>,
        _conn: &mut Connection,
    ) -> ExecResult<RespData> {
        let server = server.lock().await;
        match self {
            Config::Get(params) => {
                let mut response = Vec::with_capacity(params.len() * 2);
                for param in params {
                    match param.as_str() {
                        "DIR" => {
                            let dir = server.rdb_file.parent().unwrap_or(Path::new(""));
                            response.extend([
                                RespData::BulkString(Some(Bytes::from(param.clone()))),
                                RespData::BulkString(Some(Bytes::from(dir.display().to_string()))),
                            ]);
                        }
                        "DBFILENAME" => {
                            let filename = server.rdb_file.file_name().unwrap_or_default();
                            response.extend([
                                RespData::BulkString(Some(Bytes::from(param.clone()))),
                                RespData::BulkString(Some(Bytes::from(
                                    filename.display().to_string(),
                                ))),
                            ]);
                        }
                        _ => {}
                    }
                }
                Ok(RespData::Array(response))
            }
        }
    }
}

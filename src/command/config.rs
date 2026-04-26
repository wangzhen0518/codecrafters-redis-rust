use std::{path::Path, sync::Arc};

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

#[derive(Debug, PartialEq)]
pub enum Config {
    Get(Vec<String>),
}

impl Parse for Config {
    fn parse(args: &[Bytes]) -> ParseResult<Self>
    where
        Self: std::marker::Sized,
    {
        check_length_ge(args, 2)?;

        let subcommand = str::from_utf8(&args[0])?;
        match subcommand.to_uppercase().as_str() {
            "GET" => Ok(Config::Get(
                args[1..]
                    .iter()
                    .map(|bytes| str::from_utf8(bytes).map(|s| s.to_string()))
                    .collect::<Result<_, _>>()?,
            )),
            _ => Err(ParseError::InvalidArgument(subcommand.to_string())),
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
                    match param.to_uppercase().as_str() {
                        "DIR" => {
                            let dir = server.rdb_file.parent().unwrap_or(Path::new(""));
                            response.extend([
                                RespData::BulkString(Some(Bytes::from(param.to_string()))),
                                RespData::BulkString(Some(Bytes::from(dir.display().to_string()))),
                            ]);
                        }
                        "DBFILENAME" => {
                            let filename = server.rdb_file.file_name().unwrap_or_default();
                            response.extend([
                                RespData::BulkString(Some(Bytes::from(param.to_string()))),
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

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::Config;
    use crate::{
        command::{
            Command, ExecuteCommand, ParseError, parse_command,
            test::{build_request, build_server_connection},
        },
        resp::RespData,
    };

    #[test]
    fn parse_config_should_parse_get_params() {
        let cmd = parse_command(&build_request("CONFIG", &["GET", "dir", "dbfilename"]))
            .expect("parse config get");
        assert_eq!(
            cmd,
            Command::Config(Config::Get(vec![
                "dir".to_string(),
                "dbfilename".to_string()
            ]))
        );
    }

    #[test]
    fn parse_config_should_reject_too_few_args() {
        let err = parse_command(&build_request("CONFIG", &["get"]))
            .expect_err("config requires subcommand plus params");
        assert_eq!(
            err,
            ParseError::ExpectLengthGe(2, 1, vec![Bytes::from_owner("get")])
        );
    }

    #[test]
    fn parse_config_should_reject_invalid_subcommand() {
        let err = parse_command(&build_request("CONFIG", &["set", "x"]))
            .expect_err("unsupported config subcommand");
        assert_eq!(err, ParseError::InvalidArgument("set".to_string()));
    }

    #[tokio::test]
    async fn execute_config_get_should_return_dir_and_dbfilename() {
        let (server, mut conn) = build_server_connection().await;
        let cmd = Config::Get(vec!["dir".to_string(), "dbfilename".to_string()]);
        let resp = cmd
            .execute(server, &mut conn)
            .await
            .expect("execute config get");
        assert_eq!(
            resp,
            RespData::Array(vec![
                RespData::BulkString(Some(Bytes::from_owner("dir"))),
                RespData::BulkString(Some(Bytes::from_owner("/tmp"))),
                RespData::BulkString(Some(Bytes::from_owner("dbfilename"))),
                RespData::BulkString(Some(Bytes::from_owner("dump.rdb"))),
            ])
        );
    }

    #[tokio::test]
    async fn execute_config_get_should_ignore_unknown_parameter() {
        let (server, mut conn) = build_server_connection().await;
        let cmd = Config::Get(vec!["unknown".to_string()]);
        let resp = cmd
            .execute(server, &mut conn)
            .await
            .expect("execute config get");
        assert_eq!(resp, RespData::Array(vec![]));
    }

    #[tokio::test]
    async fn execute_config_get_should_match_case_insensitive_params() {
        let (server, mut conn) = build_server_connection().await;
        let cmd = Config::Get(vec!["DiR".to_string()]);
        let resp = cmd
            .execute(server, &mut conn)
            .await
            .expect("execute config get");
        assert_eq!(
            resp,
            RespData::Array(vec![
                RespData::BulkString(Some(Bytes::from_owner("DiR"))),
                RespData::BulkString(Some(Bytes::from_owner("/tmp"))),
            ])
        );
    }
}

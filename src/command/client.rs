use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::Mutex;

use crate::{
    command::{
        ExecuteCommand, Parse, ParseResult, check_length_eq, check_length_ge,
        error::{ExecResult, ParseError},
    },
    resp::RespData,
    server::{Connection, Server},
};

#[derive(Debug, PartialEq)]
pub enum Client {
    Info,
    GetName,
    GetReDir,
    SetInfo {
        attr_name: String,
        attr_value: String,
    },
    SetName(String),
}

impl Parse for Client {
    fn parse(args: &[Bytes]) -> ParseResult<Self>
    where
        Self: std::marker::Sized,
    {
        check_length_ge(args, 1)?;

        let subcommand = str::from_utf8(&args[0])?;
        match subcommand.to_uppercase().as_str() {
            "INFO" => {
                check_length_eq(args, 1)?;
                Ok(Client::Info)
            }
            "GETNAME" => {
                check_length_eq(args, 1)?;
                Ok(Client::GetName)
            }
            "GETREDIR" => {
                check_length_eq(args, 1)?;
                Ok(Client::GetReDir)
            }
            "SETINFO" => {
                check_length_eq(args, 3)?;
                let attr_name = str::from_utf8(&args[1])?.to_string();
                let attr_value = str::from_utf8(&args[2])?.to_string();
                Ok(Client::SetInfo {
                    attr_name,
                    attr_value,
                })
            }
            "SETNAME" => {
                check_length_eq(args, 2)?;
                let name = str::from_utf8(&args[1])?.to_string();
                Ok(Client::SetName(name))
            }
            _ => Err(ParseError::InvalidArgument(subcommand.to_string())),
        }
    }
}

impl ExecuteCommand for Client {
    async fn execute(
        &self,
        server: Arc<Mutex<Server>>,
        conn: &mut Connection,
    ) -> ExecResult<RespData> {
        let server = server.lock().await;
        match self {
            Client::Info => {
                let info_content = format!(
                    "id={id} addr={conn_addr} laddr={server_addr} \
fd=24 name={name} age=0 idle=0 flags=N db=0 sub=0 psub=0 ssub=0 multi=-1 \
watch=0 qbuf=26 qbuf-free=20448 argv-mem=10 multi-mem=0 rbs=16384 \
rbp=16384 obl=0 oll=0 omem=0 tot-mem=37786 events=r \
cmd=client|info user=default redir=-1 resp=2 \
lib-name={lib_name} lib-ver={lib_ver} io-thread=0\n",
                    id = conn.id,
                    conn_addr = conn.addr,
                    server_addr = server.addr,
                    name = conn.name,
                    lib_name = conn.lib_name,
                    lib_ver = conn.lib_ver
                );
                Ok(RespData::BulkString(Some(Bytes::from_owner(info_content))))
            }
            Client::SetInfo {
                attr_name,
                attr_value,
            } => {
                match attr_name.to_uppercase().as_str() {
                    "LIB-NAME" => conn.lib_name = attr_value.to_string(),
                    "LIB-VER" => conn.lib_ver = attr_value.to_string(),
                    _ => {
                        return Ok(RespData::BulkError(Bytes::from_owner(format!(
                            "ERR Unrecognized option '{}'",
                            attr_name
                        ))));
                    }
                }
                Ok(RespData::SimpleString("OK".to_string()))
            }
            Client::SetName(name) => {
                conn.name = name.to_string();
                Ok(RespData::SimpleString("OK".to_string()))
            }
            Client::GetName => Ok(RespData::BulkString(Some(Bytes::copy_from_slice(
                conn.name.as_bytes(),
            )))),
            Client::GetReDir => Ok(RespData::Integer(-1)),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::Client;
    use crate::{
        command::{
            Command, ExecuteCommand, ParseError, parse_command,
            test::{build_request, build_server_connection},
        },
        resp::RespData,
    };

    #[test]
    fn parse_client_should_parse_info() {
        let cmd = parse_command(&build_request("CLIENT", &["info"])).expect("parse client info");
        assert_eq!(cmd, Command::Client(Client::Info));
    }

    #[test]
    fn parse_client_should_parse_getname() {
        let cmd =
            parse_command(&build_request("CLIENT", &["getname"])).expect("parse client getname");
        assert_eq!(cmd, Command::Client(Client::GetName));
    }

    #[test]
    fn parse_client_should_parse_getredir() {
        let cmd =
            parse_command(&build_request("CLIENT", &["getredir"])).expect("parse client getredir");
        assert_eq!(cmd, Command::Client(Client::GetReDir));
    }

    #[test]
    fn parse_client_should_parse_setinfo() {
        let cmd = parse_command(&build_request("CLIENT", &["setinfo", "lib-name", "my-lib"]))
            .expect("parse client setinfo");
        assert_eq!(
            cmd,
            Command::Client(Client::SetInfo {
                attr_name: "lib-name".to_string(),
                attr_value: "my-lib".to_string(),
            })
        );
    }

    #[test]
    fn parse_client_should_parse_setname() {
        let cmd = parse_command(&build_request("CLIENT", &["setname", "alice"]))
            .expect("parse client setname");
        assert_eq!(cmd, Command::Client(Client::SetName("alice".to_string())));
    }

    #[test]
    fn parse_client_should_reject_empty_arguments() {
        let err = parse_command(&build_request("CLIENT", &[]))
            .expect_err("client needs at least one subcommand");
        assert_eq!(err, ParseError::ExpectLengthGe(1, 0, vec![]));
    }

    #[test]
    fn parse_client_should_reject_invalid_subcommand() {
        let err = parse_command(&build_request("CLIENT", &["invalid"]))
            .expect_err("invalid client subcommand");
        assert_eq!(err, ParseError::InvalidArgument("invalid".to_string()));
    }

    #[test]
    fn parse_client_should_reject_wrong_arity_for_info() {
        let err = parse_command(&build_request("CLIENT", &["info", "extra"]))
            .expect_err("info should reject extra args");
        assert_eq!(
            err,
            ParseError::ExpectLengthEq(
                1,
                2,
                vec![Bytes::from_owner("info"), Bytes::from_owner("extra")]
            )
        );
    }

    #[test]
    fn parse_client_should_reject_wrong_arity_for_setinfo() {
        let err = parse_command(&build_request("CLIENT", &["setinfo", "lib-name"]))
            .expect_err("setinfo requires two args");
        assert_eq!(
            err,
            ParseError::ExpectLengthEq(
                3,
                2,
                vec![
                    Bytes::copy_from_slice(b"setinfo"),
                    Bytes::copy_from_slice(b"lib-name"),
                ]
            )
        );
    }

    #[test]
    fn parse_client_should_reject_wrong_arity_for_getname() {
        let err = parse_command(&build_request("CLIENT", &["getname", "extra"]))
            .expect_err("getname rejects args");
        assert_eq!(
            err,
            ParseError::ExpectLengthEq(
                1,
                2,
                vec![
                    Bytes::copy_from_slice(b"getname"),
                    Bytes::copy_from_slice(b"extra")
                ]
            )
        );
    }

    #[test]
    fn parse_client_should_reject_wrong_arity_for_getredir() {
        let err = parse_command(&build_request("CLIENT", &["getredir", "extra"]))
            .expect_err("getredir rejects args");
        assert_eq!(
            err,
            ParseError::ExpectLengthEq(
                1,
                2,
                vec![
                    Bytes::copy_from_slice(b"getredir"),
                    Bytes::copy_from_slice(b"extra")
                ]
            )
        );
    }

    #[test]
    fn parse_client_should_reject_wrong_arity_for_setname() {
        let err = parse_command(&build_request("CLIENT", &["setname"]))
            .expect_err("setname requires one arg");
        assert_eq!(
            err,
            ParseError::ExpectLengthEq(2, 1, vec![Bytes::copy_from_slice(b"setname")])
        );
    }

    #[tokio::test]
    async fn execute_client_info_should_return_info_payload() {
        let (server, mut conn) = build_server_connection().await;
        conn.id = 9;
        conn.name = "alice".to_string();
        conn.lib_name = "libx".to_string();
        conn.lib_ver = "1.0.0".to_string();

        let resp = Client::Info
            .execute(server.clone(), &mut conn)
            .await
            .expect("execute client info");
        match resp {
            RespData::BulkString(Some(v)) => {
                let s = str::from_utf8(v.as_ref()).expect("info should be utf8");
                assert!(s.contains("id=9"));
                assert!(s.contains(format!("addr={}", conn.addr).as_str()));
                assert!(s.contains(format!("laddr={}", server.lock().await.addr).as_str()));
                assert!(s.contains("name=alice"));
                assert!(s.contains("lib-name=libx"));
                assert!(s.contains("lib-ver=1.0.0"));
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[tokio::test]
    async fn execute_client_setname_then_getname() {
        let (server, mut conn) = build_server_connection().await;

        let set_name = Client::SetName("alice".to_string());
        let set_resp = set_name
            .execute(server.clone(), &mut conn)
            .await
            .expect("execute client setname");
        assert_eq!(set_resp, RespData::SimpleString("OK".to_string()));

        let get_name = Client::GetName;
        let get_resp = get_name
            .execute(server, &mut conn)
            .await
            .expect("execute client getname");
        assert_eq!(
            get_resp,
            RespData::BulkString(Some(Bytes::from_owner("alice")))
        );
    }

    #[tokio::test]
    async fn execute_client_setinfo_lib_name_should_update_conn() {
        let (server, mut conn) = build_server_connection().await;
        let set_info = Client::SetInfo {
            attr_name: "LIB-NAME".to_string(),
            attr_value: "my-lib".to_string(),
        };
        let resp = set_info
            .execute(server, &mut conn)
            .await
            .expect("execute client setinfo");
        assert_eq!(resp, RespData::SimpleString("OK".to_string()));
        assert_eq!(conn.lib_name, "my-lib");
    }

    #[tokio::test]
    async fn execute_client_setinfo_lib_ver_should_update_conn() {
        let (server, mut conn) = build_server_connection().await;
        let set_info = Client::SetInfo {
            attr_name: "LIB-VER".to_string(),
            attr_value: "2.3.4".to_string(),
        };
        let resp = set_info
            .execute(server, &mut conn)
            .await
            .expect("execute client setinfo");
        assert_eq!(resp, RespData::SimpleString("OK".to_string()));
        assert_eq!(conn.lib_ver, "2.3.4");
    }

    #[tokio::test]
    async fn execute_client_setinfo_unknown_option_should_return_bulk_error() {
        let (server, mut conn) = build_server_connection().await;
        let set_info = Client::SetInfo {
            attr_name: "nope".to_string(),
            attr_value: "x".to_string(),
        };
        let resp = set_info
            .execute(server, &mut conn)
            .await
            .expect("execute client setinfo");
        assert_eq!(
            resp,
            RespData::BulkError(Bytes::from_owner("ERR Unrecognized option 'nope'"))
        );
    }

    #[tokio::test]
    async fn execute_client_getredir_should_return_minus_one() {
        let (server, mut conn) = build_server_connection().await;
        let resp = Client::GetReDir
            .execute(server, &mut conn)
            .await
            .expect("execute getredir");
        assert_eq!(resp, RespData::Integer(-1));
    }
}

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

        let subcommand = str::from_utf8(&args[0])?.to_uppercase();
        match subcommand.as_str() {
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
                let attr_name = str::from_utf8(&args[1])?.to_uppercase();
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
            _ => Err(ParseError::InvalidArgument(subcommand)),
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
                    "id={id} addr={conn_ip}:{conn_port} laddr={server_ip}:{server_port} \
fd=24 name={name} age=0 idle=0 flags=N db=0 sub=0 psub=0 ssub=0 multi=-1 \
watch=0 qbuf=26 qbuf-free=20448 argv-mem=10 multi-mem=0 rbs=16384 \
rbp=16384 obl=0 oll=0 omem=0 tot-mem=37786 events=r \
cmd=client|info user=default redir=-1 resp=2 \
lib-name={lib_name} lib-ver={lib_ver} io-thread=0\n",
                    id = conn.id,
                    conn_ip = conn.addr.ip(),
                    conn_port = conn.addr.port(),
                    server_ip = server.addr.ip(),
                    server_port = server.addr.port(),
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
                match attr_name.as_str() {
                    "LIB-NAME" => conn.lib_name = attr_value.to_string(),
                    "LIB-VER" => conn.lib_ver = attr_value.to_string(),
                    attr_name => {
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

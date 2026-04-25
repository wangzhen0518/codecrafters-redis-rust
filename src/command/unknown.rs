use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::Mutex;

use crate::{
    command::{ExecuteCommand, Parse, ParseResult, check_length_eq, error::ExecResult},
    resp::RespData,
    server::{Connection, Server},
};

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
        Ok(RespData::SimpleString("OK".to_string()))
    }
}

use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::Mutex;

use crate::{
    command::{ExecuteCommand, Parse, ParseResult, check_length_eq, error::ExecResult},
    resp::RespData,
    server::{Connection, Server},
};

pub struct Echo {
    content: Bytes,
}

impl Parse for Echo {
    fn parse(args: &[Bytes]) -> ParseResult<Self>
    where
        Self: std::marker::Sized,
    {
        check_length_eq(args, 1)?;
        Ok(Echo {
            content: args[0].clone(),
        })
    }
}

impl ExecuteCommand for Echo {
    async fn execute(
        &self,
        _server: Arc<Mutex<Server>>,
        _conn: &mut Connection,
    ) -> ExecResult<RespData> {
        Ok(RespData::BulkString(Some(self.content.clone())))
    }
}

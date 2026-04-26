use std::sync::Arc;

use bytes::Bytes;
use tokio::{sync::Mutex, time::Instant};

use crate::{
    command::{ExecuteCommand, Parse, ParseResult, check_length_eq, error::ExecResult},
    resp::RespData,
    server::{Connection, Server},
    utils::BytesInStr,
};
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
            let ((value, _)) = db.remove(&self.key).unwrap();
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

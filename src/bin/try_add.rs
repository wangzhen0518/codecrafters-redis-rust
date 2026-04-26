#![allow(unused)]

use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use bytes::{BufMut, BytesMut};

static ABC: &str = "ABC";

const MAX_NUMBER_STR_LEN: usize = u64::MAX.ilog10() as usize + 2;

fn main() {
    let x = PathBuf::from_iter([".", "a.txt"]);
    println!("{:?}", x.file_name());

    let mut buffer = BytesMut::with_capacity(0);
    buffer.put(ABC.as_bytes());
    buffer.put(ABC.as_bytes());
    dbg!(&buffer);
    dbg!(&ABC);

    let mut buf = BytesMut::zeroed(64);
    // lexical_core::BUFFER_SIZE
    {
        let new_buf = lexical_core::write(123.324, &mut buf);
        dbg!(&new_buf);
        dbg!(str::from_utf8(new_buf).unwrap());
    }
    dbg!(&buf);
    {
        let new_buf = lexical_core::write(-1, &mut buf);
        dbg!(&new_buf);
        dbg!(str::from_utf8(new_buf).unwrap());
    }
    dbg!(&buf);

    println!("s: {:?}", "123");

    println!("{}", SocketAddr::from_str("123.0.0.1:234").unwrap());
}

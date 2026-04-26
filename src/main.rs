// #![allow(warnings)]

use std::{fs, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc};

use clap::Parser;
use tokio::{net::TcpListener, sync::Mutex};

use crate::server::{Connection, Server, handle_connection};

mod command;
mod resp;
pub mod server;
mod utils;

#[derive(Debug, Parser)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(long, default_value = ".")]
    dir: String,

    #[arg(short, long, default_value = "dump.rdb")]
    dbfilename: String,

    #[arg(long, default_value = "127.0.0.1")]
    bind_source_addr: String,

    #[arg(short, long, default_value_t = 6379)]
    port: u16,
}

#[tokio::main]
async fn main() {
    utils::config_logger();

    let args = Args::parse();

    // bind tcp address
    let server_addr =
        SocketAddr::from_str(format!("{}:{}", args.bind_source_addr, args.port).as_str()).unwrap();
    let listener = TcpListener::bind(&server_addr)
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", &server_addr));

    // deal rdb file path
    let mut rdb_filename = PathBuf::from(&args.dir);
    if !rdb_filename.exists() {
        fs::create_dir(&rdb_filename)
            .unwrap_or_else(|_| panic!("Failed to create directory {}", &rdb_filename.display()));
    } else if !rdb_filename.is_dir() {
        panic!("Existing {} is not a directory.", &rdb_filename.display());
    }
    rdb_filename = fs::canonicalize(&rdb_filename).unwrap();
    rdb_filename.push(&args.dbfilename);

    // init server
    let server = Arc::new(Mutex::new(Server::new(server_addr, rdb_filename)));
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let mut s = server.lock().await;
                let conn = Connection::new(s.conn_num, addr, stream);
                s.conn_num += 1;
                tokio::spawn(handle_connection(server.clone(), conn));
            }
            Err(e) => println!("error: {}", e),
        }
    }
}

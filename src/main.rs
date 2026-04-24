use clap::Parser;
use tokio::net::TcpListener;

pub mod handler;
mod utils;

/// Self wrote redis
#[derive(Debug, Parser)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(long, default_value = "")]
    dir: String,

    #[arg(short, long, default_value = "")]
    dbfilename: String,

    #[arg(long, default_value = "127.0.0.1")]
    bind_source_addr: String,

    #[arg(long, default_value = "6379")]
    port: u16,
}

#[tokio::main]
async fn main() {
    utils::config_logger();

    let args = Args::parse();
    let listener = TcpListener::bind("127.0.0.1:6380")
        .await
        .expect("Failed to bind port 6380");
        .unwrap_or_else(|_| panic!("Failed to bind to {}", &server_addr));
        "EMPTY_NAME".to_string(),
        "EMPTY_VER".to_string(),
        args.dir,
        args.dbfilename,
    );
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => handler.handle_connection(stream, addr).await,
            Err(e) => println!("error: {}", e),
        }
    }
}

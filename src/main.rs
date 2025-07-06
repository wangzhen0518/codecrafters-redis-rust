use clap::Parser;
use tokio::net::TcpListener;

pub mod handler;
use handler::Handler;

/// Self wrote redis
#[derive(Debug, Parser)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(long, default_value = "")]
    dir: String,

    #[arg(short, long, default_value = "")]
    dbfilename: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .expect("Failed to bind port 6379");
    let handler = Handler::new(
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

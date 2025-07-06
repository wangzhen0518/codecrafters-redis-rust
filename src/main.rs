use tokio::net::TcpListener;

pub mod handler;
use handler::Handler;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .expect("Failed to bind port 6379");
    let handler = Handler::new("EMPTY_NAME".to_string(), "EMPTY_VER".to_string());
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => handler.handle_connection(stream, addr).await,
            Err(e) => println!("error: {}", e),
        }
    }
}

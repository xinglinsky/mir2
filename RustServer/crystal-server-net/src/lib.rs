use std::{io, net::SocketAddr, sync::Arc};

use crystal_shared_proto::packet::RawPacket;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub trait ConnectionHandler: Send + 'static {
    /// Called right after a TCP connection is accepted, before any packets are read.
    /// Can return zero or more raw responses to send immediately (e.g. handshake).
    fn on_connect(&mut self) -> Vec<Vec<u8>> {
        Vec::new()
    }

    /// Consume an incoming packet and return zero or more raw responses
    /// that should be written back to the client immediately.
    fn handle_packet(&mut self, packet: RawPacket) -> Vec<Vec<u8>>;
}

pub type HandlerFactory = Arc<dyn Fn() -> Box<dyn ConnectionHandler> + Send + Sync + 'static>;

pub async fn run_server(addr: SocketAddr, factory: HandlerFactory) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let factory = factory.clone();

        tokio::spawn(async move {
            let _ = handle_connection(stream, factory).await;
        });
    }
}

async fn handle_connection(mut stream: TcpStream, factory: HandlerFactory) -> io::Result<()> {
    let mut handler = factory();
    let initial = handler.on_connect();
    for response in initial {
        stream.write_all(&response).await?;
    }
    let mut buf = Vec::new();
    let mut read_buf = [0u8; 4096];

    loop {
        let n = stream.read(&mut read_buf).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&read_buf[..n]);

        loop {
            match RawPacket::decode(&buf) {
                Some((packet, remaining)) => {
                    let responses = handler.handle_packet(packet);
                    for response in responses {
                        stream.write_all(&response).await?;
                    }
                    buf = remaining.to_vec();
                }
                None => break,
            }
        }
    }

    Ok(())
}

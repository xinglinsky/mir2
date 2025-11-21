use std::{io, net::SocketAddr, sync::Arc, time::Duration};

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

    /// Called regularly by the transport layer even when no packets are read,
    /// so the handler can emit unsolicited responses (e.g. broadcasts or
    /// world events) to the client.
    fn poll_outbound(&mut self) -> Vec<Vec<u8>> {
        Vec::new()
    }

    fn should_close(&self) -> bool {
        false
    }

    /// Called once when the TCP connection is closed (EOF or error) so the
    /// handler can perform any necessary cleanup or persistence. Any
    /// responses returned here are ignored by the transport layer.
    fn on_disconnect(&mut self) {}
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
    stream.set_nodelay(true)?;

    let mut handler = factory();
    let initial = handler.on_connect();
    for response in initial {
        if let Err(e) = stream.write_all(&response).await {
            eprintln!("[net] write error during on_connect: {}", e);
            // Even if the initial write fails, treat this as a disconnect so
            // the handler can persist any partial state.
            handler.on_disconnect();
            return Ok(());
        }
    }
    let mut buf = Vec::new();
    let mut read_buf = [0u8; 4096];

    loop {
        // Try to read with a small timeout so that even if the client is idle
        // we still get a chance to poll for outbound data to send.
        let read_result =
            tokio::time::timeout(Duration::from_millis(50), stream.read(&mut read_buf)).await;

        match read_result {
            Ok(Ok(0)) => {
                // Clean EOF from the client: treat as a normal disconnect.
                break;
            }
            Ok(Ok(n)) => {
                buf.extend_from_slice(&read_buf[..n]);

                loop {
                    match RawPacket::decode(&buf) {
                        Some((packet, remaining)) => {
                            let responses = handler.handle_packet(packet);
                            for response in responses {
                                if let Err(e) = stream.write_all(&response).await {
                                    // Write error while sending a response; log and
                                    // treat this as a disconnect. Call on_disconnect
                                    // immediately so we don't lose any in-memory
                                    // state for the active character.
                                    eprintln!("[net] write error to client: {}", e);
                                    handler.on_disconnect();
                                    return Ok(());
                                }
                            }
                            buf = remaining.to_vec();
                        }
                        None => break,
                    }
                }
            }
            Ok(Err(e)) => {
                // Socket read error (e.g. connection reset by peer). Log and
                // break so we can still call on_disconnect below instead of
                // returning early and skipping persistence.
                eprintln!("[net] read error from client: {}", e);
                break;
            }
            Err(_elapsed) => {
                // Timed out waiting for client data; fall through to
                // poll_outbound below.
            }
        }

        let outbound = handler.poll_outbound();
        for response in outbound {
            if let Err(e) = stream.write_all(&response).await {
                // Write error while sending a response; log and
                // treat this as a disconnect. Call on_disconnect
                // immediately so we don't lose any in-memory
                // state for the active character.
                eprintln!("[net] write error to client: {}", e);
                handler.on_disconnect();
                return Ok(());
            }
        }

        if handler.should_close() {
            break;
        }
    }

    // Notify the handler that the connection is closing so it can persist
    // any in-memory state (e.g. character position) if desired.
    handler.on_disconnect();

    Ok(())
}

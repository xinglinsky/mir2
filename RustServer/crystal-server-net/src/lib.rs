use std::{
    collections::HashMap,
    io,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crystal_shared_proto::packet::RawPacket;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

struct IpState {
    blocked_until: Option<Instant>,
    active: u16,
}

struct IpLimiter {
    max_ip: u16,
    block_duration: Duration,
    states: Mutex<HashMap<IpAddr, IpState>>,
}

impl IpLimiter {
    fn new(max_ip: u16, block_duration: Duration) -> Self {
        IpLimiter {
            max_ip,
            block_duration,
            states: Mutex::new(HashMap::new()),
        }
    }

    /// Attempt to acquire a connection slot for the given IP address.
    /// Returns true if the connection is allowed, or false if the IP is
    /// currently blocked or has reached the maximum concurrent connection
    /// count.
    fn try_acquire(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut map = self.states.lock().unwrap();
        let entry = map.entry(ip).or_insert(IpState {
            blocked_until: None,
            active: 0,
        });

        if let Some(until) = entry.blocked_until {
            if until > now {
                return false;
            }
        }

        if self.max_ip > 0 && entry.active >= self.max_ip {
            entry.blocked_until = Some(now + self.block_duration);
            return false;
        }

        entry.active = entry.active.saturating_add(1);
        true
    }

    fn release(&self, ip: IpAddr) {
        let mut map = match self.states.lock() {
            Ok(m) => m,
            Err(_) => return,
        };
        if let Some(entry) = map.get_mut(&ip) {
            if entry.active > 0 {
                entry.active -= 1;
            }
        }
    }
}

struct IpGuard {
    limiter: Arc<IpLimiter>,
    ip: IpAddr,
}

impl Drop for IpGuard {
    fn drop(&mut self) {
        self.limiter.release(self.ip);
    }
}

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

pub async fn run_server(
    addr: SocketAddr,
    factory: HandlerFactory,
    max_ip: u16,
    ip_block_seconds: u64,
) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let block_secs = if ip_block_seconds == 0 { 1 } else { ip_block_seconds };
    let limiter = Arc::new(IpLimiter::new(max_ip, Duration::from_secs(block_secs)));

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let factory = factory.clone();
        let limiter = limiter.clone();

        tokio::spawn(async move {
            let ip = peer_addr.ip();
            if !limiter.try_acquire(ip) {
                eprintln!(
                    "[net] rejecting connection from {} due to IP limits or temporary block",
                    ip
                );
                return;
            }

            let _ = handle_connection(stream, factory, limiter, ip).await;
        });
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    factory: HandlerFactory,
    limiter: Arc<IpLimiter>,
    ip: IpAddr,
) -> io::Result<()> {
    let _guard = IpGuard { limiter, ip };
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

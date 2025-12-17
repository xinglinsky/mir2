use std::io;

use crystal_shared_proto::packet::RawPacket;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum NetEvent {
    Connected,
    Disconnected,
    Packet(RawPacket),
    Error(String),
}

pub struct NetClient {
    outbound: mpsc::Sender<Vec<u8>>,
    inbound: mpsc::Receiver<NetEvent>,
    _rt: Runtime,
}

impl NetClient {
    pub fn connect(addr: &str) -> io::Result<Self> {
        let rt = Runtime::new()?;
        let addr = addr.to_string();

        let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>(1024);
        let (in_tx, in_rx) = mpsc::channel::<NetEvent>(1024);

        rt.spawn(async move {
            match TcpStream::connect(&addr).await {
                Ok(stream) => {
                    if let Err(e) = stream.set_nodelay(true) {
                        let _ = in_tx
                            .send(NetEvent::Error(format!("set_nodelay error: {e}")))
                            .await;
                        let _ = in_tx.send(NetEvent::Disconnected).await;
                        return;
                    }
                    run_connection(stream, out_rx, in_tx).await;
                }
                Err(e) => {
                    let _ = in_tx.send(NetEvent::Error(format!("connect error: {e}"))).await;
                    let _ = in_tx.send(NetEvent::Disconnected).await;
                }
            }
        });

        Ok(NetClient {
            outbound: out_tx,
            inbound: in_rx,
            _rt: rt,
        })
    }

    pub fn try_recv(&mut self) -> Option<NetEvent> {
        self.inbound.try_recv().ok()
    }

    pub fn send_raw(&self, pkt: RawPacket) -> Result<(), mpsc::error::TrySendError<Vec<u8>>> {
        self.outbound.try_send(pkt.encode())
    }
}

async fn run_connection(
    mut stream: TcpStream,
    mut outbound: mpsc::Receiver<Vec<u8>>,
    inbound: mpsc::Sender<NetEvent>,
) {
    let _ = inbound.send(NetEvent::Connected).await;

    let mut buf: Vec<u8> = Vec::new();
    let mut read_buf = [0u8; 4096];

    loop {
        tokio::select! {
            read_res = stream.read(&mut read_buf) => {
                match read_res {
                    Ok(0) => {
                        let _ = inbound.send(NetEvent::Disconnected).await;
                        break;
                    }
                    Ok(n) => {
                        buf.extend_from_slice(&read_buf[..n]);
                        loop {
                            match RawPacket::decode(&buf) {
                                Some((packet, remaining)) => {
                                    let _ = inbound.send(NetEvent::Packet(packet)).await;
                                    buf = remaining.to_vec();
                                }
                                None => break,
                            }
                        }
                    }
                    Err(e) => {
                        let _ = inbound.send(NetEvent::Error(format!("read error: {e}"))).await;
                        let _ = inbound.send(NetEvent::Disconnected).await;
                        break;
                    }
                }
            }
            out = outbound.recv() => {
                match out {
                    Some(bytes) => {
                        if let Err(e) = stream.write_all(&bytes).await {
                            let _ = inbound.send(NetEvent::Error(format!("write error: {e}"))).await;
                            let _ = inbound.send(NetEvent::Disconnected).await;
                            break;
                        }
                    }
                    None => {
                        let _ = inbound.send(NetEvent::Disconnected).await;
                        break;
                    }
                }
            }
        }
    }
}

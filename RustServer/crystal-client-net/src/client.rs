//! 网络客户端核心实现

use crate::events::NetEvent;
use crate::reconnect::ReconnectStrategy;
use crate::metrics::NetworkMetrics;
use crystal_shared_proto::packet::RawPacket;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

/// 网络客户端
///
/// 负责管理与服务器的 TCP 连接，处理数据包的收发。
pub struct NetClient {
    outbound: mpsc::Sender<Vec<u8>>,
    inbound: mpsc::Receiver<NetEvent>,
    _rt: Runtime,
    reconnect_strategy: ReconnectStrategy,
    metrics: NetworkMetrics,
}

impl NetClient {
    /// 连接到服务器
    ///
    /// # Arguments
    ///
    /// * `addr` - 服务器地址（格式：IP:Port）
    ///
    /// # Returns
    ///
    /// 成功返回 `NetClient`，失败返回 IO 错误
    pub fn connect(addr: &str) -> io::Result<Self> {
        Self::connect_with_strategy(addr, ReconnectStrategy::default())
    }

    /// 使用自定义重连策略连接到服务器
    ///
    /// # Arguments
    ///
    /// * `addr` - 服务器地址
    /// * `strategy` - 重连策略
    pub fn connect_with_strategy(addr: &str, strategy: ReconnectStrategy) -> io::Result<Self> {
        let rt = Runtime::new()?;
        let addr = addr.to_string();
        let metrics = NetworkMetrics::default();

        let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>(1024);
        let (in_tx, in_rx) = mpsc::channel::<NetEvent>(1024);

        let metrics_clone = metrics.clone();
        let strategy_clone = strategy.clone();
        rt.spawn(async move {
            Self::run_connection_loop(addr, out_rx, in_tx, strategy, metrics_clone).await;
        });

        Ok(NetClient {
            outbound: out_tx,
            inbound: in_rx,
            _rt: rt,
            reconnect_strategy: strategy_clone,
            metrics,
        })
    }

    /// 尝试接收网络事件（非阻塞）
    ///
    /// # Returns
    ///
    /// 如果有事件则返回 `Some(NetEvent)`，否则返回 `None`
    pub fn try_recv(&mut self) -> Option<NetEvent> {
        self.inbound.try_recv().ok()
    }

    /// 发送原始数据包
    ///
    /// # Arguments
    ///
    /// * `pkt` - 要发送的数据包
    ///
    /// # Returns
    ///
    /// 成功返回 `Ok(())`，失败返回发送错误
    pub fn send_raw(&self, pkt: RawPacket) -> Result<(), mpsc::error::TrySendError<Vec<u8>>> {
        let bytes = pkt.encode();
        let len = bytes.len();
        match self.outbound.try_send(bytes) {
            Ok(()) => {
                self.metrics.record_sent(len);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// 获取网络指标
    pub fn metrics(&self) -> &NetworkMetrics {
        &self.metrics
    }

    /// 获取重连策略
    pub fn reconnect_strategy(&self) -> &ReconnectStrategy {
        &self.reconnect_strategy
    }

    async fn run_connection_loop(
        addr: String,
        mut outbound: mpsc::Receiver<Vec<u8>>,
        inbound: mpsc::Sender<NetEvent>,
        mut strategy: ReconnectStrategy,
        mut metrics: NetworkMetrics,
    ) {
        loop {
            // 尝试连接
            match TcpStream::connect(&addr).await {
                Ok(stream) => {
                    if let Err(e) = stream.set_nodelay(true) {
                        let _ = inbound
                            .send(NetEvent::Error(format!("set_nodelay error: {e}")))
                            .await;
                        let _ = inbound.send(NetEvent::Disconnected).await;
                        break;
                    } else {
                        metrics.record_connection();
                        // 直接使用 outbound receiver（注意：这会在连接断开时消耗掉 receiver）
                        // 如果需要重连，我们需要重新设计架构，让外部管理 receiver
                        let need_reconnect = Self::run_connection(stream, &mut outbound, inbound.clone(), &mut metrics).await;
                        
                        if need_reconnect {
                            // 连接断开，检查是否需要重连
                            if !strategy.should_reconnect() {
                                break;
                            }
                            
                            // 等待重连延迟
                            let delay = strategy.next_delay();
                            let _ = inbound.send(NetEvent::Error(format!("Reconnecting in {}ms...", delay.as_millis()))).await;
                            tokio::time::sleep(delay).await;
                            strategy.record_attempt();
                        } else {
                            // 正常断开，不重连
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = inbound.send(NetEvent::Error(format!("connect error: {e}"))).await;
                    let _ = inbound.send(NetEvent::Disconnected).await;
                    
                    // 检查是否需要重连
                    if !strategy.should_reconnect() {
                        break;
                    }
                    
                    let delay = strategy.next_delay();
                    tokio::time::sleep(delay).await;
                    strategy.record_attempt();
                }
            }
        }
    }

    async fn run_connection(
        mut stream: TcpStream,
        outbound: &mut mpsc::Receiver<Vec<u8>>,
        inbound: mpsc::Sender<NetEvent>,
        metrics: &mut NetworkMetrics,
    ) -> bool {
        let _ = inbound.send(NetEvent::Connected).await;

        let mut buf: Vec<u8> = Vec::new();
        let mut read_buf = [0u8; 4096];

        loop {
            tokio::select! {
                read_res = stream.read(&mut read_buf) => {
                    match read_res {
                        Ok(0) => {
                            metrics.record_disconnection();
                            let _ = inbound.send(NetEvent::Disconnected).await;
                            return false; // 正常断开，不需要重连
                        }
                        Ok(n) => {
                            metrics.record_received(n);
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
                            metrics.record_error();
                            metrics.record_disconnection();
                            let _ = inbound.send(NetEvent::Error(format!("read error: {e}"))).await;
                            let _ = inbound.send(NetEvent::Disconnected).await;
                            return true; // 错误断开，需要重连
                        }
                    }
                }
                out = outbound.recv() => {
                    match out {
                        Some(bytes) => {
                            if let Err(e) = stream.write_all(&bytes).await {
                                metrics.record_error();
                                metrics.record_disconnection();
                                let _ = inbound.send(NetEvent::Error(format!("write error: {e}"))).await;
                                let _ = inbound.send(NetEvent::Disconnected).await;
                                return true; // 错误断开，需要重连
                            }
                        }
                        None => {
                            metrics.record_disconnection();
                            let _ = inbound.send(NetEvent::Disconnected).await;
                            return false; // 正常断开，不需要重连
                        }
                    }
                }
            }
        }
    }
}


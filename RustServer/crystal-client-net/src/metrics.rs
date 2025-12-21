//! 网络指标统计

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// 网络指标
///
/// 跟踪网络连接的统计信息，包括发送/接收的字节数、连接次数等
#[derive(Clone, Default)]
pub struct NetworkMetrics {
    /// 发送的字节总数
    bytes_sent: Arc<AtomicU64>,
    
    /// 接收的字节总数
    bytes_received: Arc<AtomicU64>,
    
    /// 发送的数据包总数
    packets_sent: Arc<AtomicU64>,
    
    /// 接收的数据包总数
    packets_received: Arc<AtomicU64>,
    
    /// 连接次数
    connection_count: Arc<AtomicU64>,
    
    /// 断开次数
    disconnection_count: Arc<AtomicU64>,
    
    /// 错误次数
    error_count: Arc<AtomicU64>,
    
    /// 连接开始时间
    connected_at: Arc<std::sync::Mutex<Option<Instant>>>,
}

impl NetworkMetrics {
    /// 创建新的网络指标
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录发送的字节数
    pub fn record_sent(&self, bytes: usize) {
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录接收的字节数
    pub fn record_received(&self, bytes: usize) {
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
        self.packets_received.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录连接
    pub fn record_connection(&self) {
        self.connection_count.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut connected_at) = self.connected_at.lock() {
            *connected_at = Some(Instant::now());
        }
    }

    /// 记录断开
    pub fn record_disconnection(&self) {
        self.disconnection_count.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut connected_at) = self.connected_at.lock() {
            *connected_at = None;
        }
    }

    /// 记录错误
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取发送的字节总数
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// 获取接收的字节总数
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    /// 获取发送的数据包总数
    pub fn packets_sent(&self) -> u64 {
        self.packets_sent.load(Ordering::Relaxed)
    }

    /// 获取接收的数据包总数
    pub fn packets_received(&self) -> u64 {
        self.packets_received.load(Ordering::Relaxed)
    }

    /// 获取连接次数
    pub fn connection_count(&self) -> u64 {
        self.connection_count.load(Ordering::Relaxed)
    }

    /// 获取断开次数
    pub fn disconnection_count(&self) -> u64 {
        self.disconnection_count.load(Ordering::Relaxed)
    }

    /// 获取错误次数
    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    /// 获取连接持续时间（如果已连接）
    pub fn connection_duration(&self) -> Option<Duration> {
        if let Ok(connected_at) = self.connected_at.lock() {
            connected_at.map(|t| t.elapsed())
        } else {
            None
        }
    }

    /// 重置所有指标
    pub fn reset(&self) {
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.packets_sent.store(0, Ordering::Relaxed);
        self.packets_received.store(0, Ordering::Relaxed);
        self.connection_count.store(0, Ordering::Relaxed);
        self.disconnection_count.store(0, Ordering::Relaxed);
        self.error_count.store(0, Ordering::Relaxed);
    }
}

use std::time::Duration;


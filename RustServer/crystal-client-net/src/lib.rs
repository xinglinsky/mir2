//! Crystal Client 网络层
//!
//! 本 crate 提供客户端网络连接管理，包括：
//! - TCP 连接管理
//! - 数据包收发
//! - 重连策略
//! - 网络指标统计

mod client;
mod events;
mod reconnect;
mod metrics;

pub use client::NetClient;
pub use events::NetEvent;
pub use reconnect::ReconnectStrategy;
pub use metrics::NetworkMetrics;

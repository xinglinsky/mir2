//! 网络事件定义

use crystal_shared_proto::packet::RawPacket;

/// 网络事件
///
/// 表示网络连接状态变化或接收到数据包的事件
#[derive(Debug, Clone)]
pub enum NetEvent {
    /// 已连接到服务器
    Connected,
    
    /// 已断开连接
    Disconnected,
    
    /// 接收到数据包
    Packet(RawPacket),
    
    /// 发生错误
    Error(String),
}


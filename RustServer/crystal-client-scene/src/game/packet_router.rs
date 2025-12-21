//! 游戏场景包路由

use bevy::prelude::*;
use crystal_client_net::NetEvent;
use crystal_shared_proto::packet::RawPacket;
use crystal_shared_proto::login::ServerPacketId;

/// 游戏场景包路由系统
pub fn route_game_packets(
    // TODO: 接收网络事件并路由到对应的处理器
) {
    // TODO: 实现包路由逻辑
    // - 接收 NetEvent::Packet
    // - 根据 packet.id 分发到不同的 handler
    // - 更新游戏世界状态
}


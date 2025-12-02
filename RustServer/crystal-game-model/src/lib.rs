//! Shared game model types for Crystal server/client.
//! 当前仅作为占位和未来 ECS/双端共享数据结构的起点，暂未被任何 crate 使用。

/// 逻辑坐标（格子为单位）。
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// 简单朝向表示（与客户端现有协议含义保持一致）。
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Direction2D {
    pub dir: u8,
}

/// 基础属性集合，仅作示例，后续可根据实际需要扩展/调整。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BasicAttributes {
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub attack_min: i32,
    pub attack_max: i32,
    pub defence: i32,
    pub magic_defence: i32,
}

// 目前不会在任何 crate 中直接使用这些类型，
// 仅作为未来共享模型/ECS 化的基础占位定义。

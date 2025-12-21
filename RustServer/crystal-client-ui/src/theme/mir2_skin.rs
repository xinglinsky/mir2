//! Mir2 按钮皮肤管理
//!
//! 管理 Title/Prguse 等 Lib 文件中的按钮三态（base/hover/pressed）索引。

use bevy::prelude::*;
use crystal_client_asset::libset_module::LibId;

/// 按钮三态皮肤
#[derive(Clone, Debug)]
pub struct ButtonSkin {
    /// 基础状态（正常）
    pub base: Handle<Image>,
    /// 悬停状态
    pub hover: Handle<Image>,
    /// 按下状态
    pub pressed: Handle<Image>,
}

/// 按钮皮肤管理器
///
/// 管理不同 Lib 文件中的按钮皮肤索引。
pub struct ButtonSkinManager;

impl ButtonSkinManager {
    /// 从 Lib 文件中加载按钮皮肤
    ///
    /// # 参数
    /// - `lib_id`: Lib 文件 ID（如 `LibId::Title` 或 `LibId::Prguse`）
    /// - `base_index`: 基础状态图片索引
    /// - `hover_index`: 悬停状态图片索引
    /// - `pressed_index`: 按下状态图片索引
    ///
    /// # 返回
    /// 返回 `ButtonSkin`，如果索引无效则返回 `None`
    pub fn load_skin(
        _lib_id: LibId,
        _base_index: usize,
        _hover_index: usize,
        _pressed_index: usize,
    ) -> Option<ButtonSkin> {
        // TODO: 从 LibStore 加载图片并创建 ButtonSkin
        None
    }
}


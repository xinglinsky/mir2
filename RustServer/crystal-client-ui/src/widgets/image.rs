//! Image Widget
//!
//! 从 Lib 文件索引创建 Bevy Image 的 Widget。

use bevy::prelude::*;
use crystal_client_asset::libset_module::LibId;

/// Image Widget 组件
///
/// 用于从 Lib 文件索引创建 UI 图片。
#[derive(Component)]
pub struct ImageWidget {
    /// Lib 文件 ID
    pub lib_id: LibId,
    /// 图片索引
    pub index: usize,
}

/// 更新 Image Widget 系统
pub fn update_image_widget(
    mut query: Query<(&ImageWidget, &mut UiImage), Changed<ImageWidget>>,
    // TODO: 需要 LibStore 来获取图片
) {
    for (widget, mut ui_image) in query.iter_mut() {
        // TODO: 从 LibStore 获取图片并更新 UiImage
        // let image_handle = lib_store.get_image(widget.lib_id, widget.index)?;
        // ui_image.texture = image_handle;
    }
}


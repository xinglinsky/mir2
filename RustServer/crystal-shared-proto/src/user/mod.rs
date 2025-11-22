// User module split façade for large user.rs.
// Follows the RUST_REWRITE_PLAN "user/" structure by grouping types by responsibility
// while keeping the original flat definitions in a private submodule.

mod flat;

pub mod information;
pub mod location;
pub mod scene_object;
pub mod status;
pub mod group;
pub mod system;

// Keep existing public API: crate::user::SUserInformation, etc.
// 基础信息和位移相关类型从对应子模块导出，其余暂时仍从 flat 透出。
pub use information::{SUserInformation, SUserSlotsRefresh};
pub use location::{
    SUserLocation,
    SUserDash,
    SUserDashFail,
    SUserBackStep,
    SUserDashAttack,
    SUserAttackMove,
};
pub use status::{SDamageIndicator, SHealthChanged, SHeroHealthChanged, SStruck, SColourChanged};
pub use flat::*;

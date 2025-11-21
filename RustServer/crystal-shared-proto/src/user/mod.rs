// User module split façade for large user.rs.
// Follows the RUST_REWRITE_PLAN "user/" structure by grouping types by responsibility
// while keeping the original flat definitions in a private submodule.

mod flat;

// Keep existing public API: crate::user::SUserInformation, etc.
pub use flat::*;

pub mod information;
pub mod location;
pub mod scene_object;
pub mod status;

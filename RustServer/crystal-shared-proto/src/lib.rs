// crystal-shared-proto: Rust reimplementation of Suprcode/Crystal Shared packet protocol.
// For now this is just a stub; we'll gradually add packet framing and message structs.

pub mod packet;
pub mod io;
pub mod login;
pub mod select;
pub mod map;
pub mod user;
pub mod item;
pub mod quest;
pub mod shop;
pub mod mail;
pub mod scene;
pub mod npc;

use crate::world::{SessionId};
use crate::world::types::PoisonType;

#[derive(Clone, Debug)]
pub struct PoisonInstance {
    pub owner_session_id: Option<SessionId>,
    pub poison_type: PoisonType,
    pub value: i32,
    pub duration: i64,
    pub time: i64,
    pub tick_time_ms: i64,
    pub tick_speed_ms: i64,
}

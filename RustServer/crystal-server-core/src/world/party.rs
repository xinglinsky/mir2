use std::collections::HashMap;

use super::SessionId;

pub type PartyId = u32;

#[derive(Clone, Debug)]
pub struct Party {
    pub id: PartyId,
    pub leader: SessionId,
    pub members: Vec<SessionId>,
}

#[derive(Clone, Debug, Default)]
pub struct PartyManager {
    pub next_id: PartyId,
    pub parties: HashMap<PartyId, Party>,
}

impl PartyManager {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            parties: HashMap::new(),
        }
    }
}

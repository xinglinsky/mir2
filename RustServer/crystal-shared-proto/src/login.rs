// Login-related protocol messages compatible with the existing C# Crystal implementation.
// This module focuses on the minimal set needed for client/server login and start-game/character management flow.
mod ids;
mod client;
mod server;

pub use ids::{ClientPacketId, ServerPacketId};
pub use client::*;
pub use server::*;
pub use crate::item::{CMoveItem, CEquipItem, CRemoveItem, CUseItem, CDropItem, CStoreItem, CTakeBackItem, CRemoveSlotItem, CSplitItem, CDropGold};
pub use crate::npc::{
    CCallNPC,
    CBuyItem,
    CSellItem,
    CDepositRefineItem,
    CRetrieveRefineItem,
    CRefineCancel,
    CRefineItem,
    CCheckRefine,
    CReplaceWedRing,
    CDepositTradeItem,
    CRetrieveTradeItem,
};
pub use crate::guild::{CGuildInvite, CGuildNameReturn, CEditGuildMember};
pub use crate::hero::{
    CChangeHero,
    CNewHero,
    CSetAutoPotItem,
    CSetAutoPotValue,
    CSetHeroBehaviour,
    CTakeBackHeroItem,
    CTransferHeroItem,
};

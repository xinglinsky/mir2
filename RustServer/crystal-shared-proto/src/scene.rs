// Scene / object / combat related packets.
// These types are primarily defined in user.rs and npc.rs and re-exported here
// to group scene-related functionality under a dedicated module, similar to
// the C# GameScene organisation.

pub use crate::user::{
    SChat,
    SDeath,
    SDamageIndicator,
    SGainExperience,
    SGainHeroExperience,
    SGainedCredit,
    SGainedGold,
    SGainedItem,
    SHeroHealthChanged,
    SHeroLevelChanged,
    SLevelChanged,
    SLoseCredit,
    SLoseGold,
    SObjectAttack,
    SObjectChat,
    SObjectDied,
    SObjectGold,
    SObjectHero,
    SObjectItem,
    SObjectLeveled,
    SObjectMonster,
    SObjectPlayer,
    SObjectRemove,
    SObjectStruck,
    SObjectRun,
    SObjectTurn,
    SObjectTurnWalkRun,
    SObjectWalk,
    SStruck,
    SObjectTeleportOut,
    SObjectTeleportIn,
    STeleportIn,
};

pub use crate::npc::{SObjectNpc, SNpcResponse};

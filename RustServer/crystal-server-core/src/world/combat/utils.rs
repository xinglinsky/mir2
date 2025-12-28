//! Combat utility functions
//!
//! This module provides utility functions for combat-related checks:
//! - Attack target validation (PvP and PvE)
//! - Attack permission checks

use crate::world::player::PlayerState;
use crate::world::provider::WorldProvider;
use crate::world::types::AttackMode;
use crate::world::{SessionId, World};

/// Check if a player can attack (basic check - not dead, has HP)
pub fn can_attack(player: &PlayerState) -> bool {
    !player.dead && player.hp > 0
}

impl<P: WorldProvider> World<P> {
    /// Determine whether `attacker_sid` is allowed to attack `target_sid`
    /// according to the C# PlayerObject.IsAttackTarget(HumanObject attacker)
    /// rules. This inspects map NoFight, SafeZone membership and the
    /// attacker's AttackMode, together with party and guild membership and
    /// the target's PK status (red/brown).
    pub(crate) fn can_attack_player(&self, attacker_sid: SessionId, target_sid: SessionId) -> bool {
        if attacker_sid == target_sid {
            return false;
        }

        let attacker = match self.players.get(&attacker_sid) {
            Some(p) => p,
            None => return false,
        };
        let target = match self.players.get(&target_sid) {
            Some(p) => p,
            None => return false,
        };

        if attacker.dead || target.dead || attacker.hp <= 0 || target.hp <= 0 {
            return false;
        }

        if attacker.map_index != target.map_index {
            return false;
        }
        let map_index = attacker.map_index;

        let map_info = match self.provider.get_map_info(map_index) {
            Some(info) => info,
            None => return false,
        };

        // Mirror C# CurrentMap.Info.NoFight and InSafeZone checks.
        if map_info.no_fight {
            return false;
        }

        let attacker_in_safe = Self::point_in_safe_zone(map_info, attacker.x, attacker.y);
        let target_in_safe = Self::point_in_safe_zone(map_info, target.x, target.y);
        if attacker_in_safe || target_in_safe {
            return false;
        }

        let mode = AttackMode::from_u8(attacker.attack_mode);

        match mode {
            AttackMode::Peace => false,
            AttackMode::All => true,
            AttackMode::Group => {
                // C#: Group => GroupMembers == null || !GroupMembers.Contains(attacker)
                // Here: only disallow when both are in the same party.
                if let (Some(a_pid), Some(t_pid)) = (attacker.party_id, target.party_id) {
                    a_pid != t_pid
                } else {
                    true
                }
            }
            AttackMode::Guild => {
                // C#: Guild => MyGuild == null || MyGuild != attacker.MyGuild
                // i.e. cannot attack same-guild members when both have guilds.
                if target.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    true
                } else {
                    attacker.guild_name != target.guild_name
                }
            }
            AttackMode::EnemyGuild => {
                // C#: EnemyGuild => MyGuild != null && MyGuild.IsEnemy(attacker.MyGuild)
                // Until full guild war state is implemented, approximate this
                // as: both have non-empty, different guild names.
                if target.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    false
                } else {
                    attacker.guild_name != target.guild_name
                }
            }
            AttackMode::RedBrown => {
                // C#: RedBrown => PKPoints >= 200 || Envir.Time < BrownTime
                target.pk_points >= 200 || self.time_ms < target.brown_time_ms
            }
        }
    }

    /// Determine whether `attacker_sid` is allowed to attack the specified
    /// monster according to the C# MonsterObject.IsAttackTarget(HumanObject
    /// attacker) rules. This primarily affects pets (monsters with a player
    /// owner) and mirrors the interaction with AttackMode/party/guild/PK
    /// state, while leaving wild monsters unrestricted by AttackMode.
    pub(crate) fn can_attack_monster(
        &self,
        attacker_sid: SessionId,
        map_index: i32,
        monster_id: u64,
    ) -> bool {
        let attacker = match self.players.get(&attacker_sid) {
            Some(p) => p,
            None => return false,
        };

        let monsters = match self.monsters.get(&map_index) {
            Some(ms) => ms,
            None => return false,
        };

        let monster = match monsters.iter().find(|m| m.id == monster_id && m.hp > 0) {
            Some(m) => m,
            None => return false,
        };

        // Wild monsters (no owner) are always valid targets regardless of
        // AttackMode; map-level NoFight/SafeZone rules are handled elsewhere.
        let owner_sid = match monster.owner_session_id {
            Some(sid) => sid,
            None => return true,
        };

        let owner = match self.players.get(&owner_sid) {
            Some(p) => p,
            // If the owner is missing from world state, fall back to treating
            // this as a wild monster.
            None => return true,
        };

        let mode = AttackMode::from_u8(attacker.attack_mode);

        // C#: if (attacker.AMode == AttackMode.Peace) return false; (for pets)
        if mode == AttackMode::Peace {
            return false;
        }

        // C#: if (Master == attacker) return attacker.AMode == AttackMode.All;
        if owner_sid == attacker_sid {
            return mode == AttackMode::All;
        }

        // Approximate the C# safe-zone rule for pets:
        // if (Master.Race == ObjectType.Player && (attacker.InSafeZone || InSafeZone)) return false;
        if let Some(map_info) = self.provider.get_map_info(map_index) {
            let attacker_in_safe =
                Self::point_in_safe_zone(map_info, attacker.x, attacker.y);
            let monster_in_safe =
                Self::point_in_safe_zone(map_info, monster.x, monster.y);
            if attacker_in_safe || monster_in_safe {
                return false;
            }
        }

        match mode {
            AttackMode::All => true,
            AttackMode::Group => {
                // C#: Group => Master.GroupMembers == null || !Master.GroupMembers.Contains(attacker)
                // Approximate via shared PartyId: disallow when both are in
                // the same party.
                if let (Some(a_pid), Some(o_pid)) = (attacker.party_id, owner.party_id) {
                    a_pid != o_pid
                } else {
                    true
                }
            }
            AttackMode::Guild => {
                // C#: Guild => master.MyGuild == null || master.MyGuild != attacker.MyGuild
                if owner.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    true
                } else {
                    owner.guild_name != attacker.guild_name
                }
            }
            AttackMode::EnemyGuild => {
                // C#: EnemyGuild => master.MyGuild != null && attacker.MyGuild != null && master.MyGuild.IsEnemy(attacker.MyGuild)
                // Approximate as: both have non-empty, different guild names.
                if owner.guild_name.is_empty() || attacker.guild_name.is_empty() {
                    false
                } else {
                    owner.guild_name != attacker.guild_name
                }
            }
            AttackMode::RedBrown => {
                // C#: RedBrown => Master.PKPoints >= 200 || Envir.Time < Master.BrownTime
                owner.pk_points >= 200 || self.time_ms < owner.brown_time_ms
            }
            AttackMode::Peace => false, // already handled above
        }
    }
}


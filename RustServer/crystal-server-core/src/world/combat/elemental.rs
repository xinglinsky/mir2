//! Elemental damage system
//! 
//! Handles calculation and application of elemental damage (Fire, Ice, Lightning, etc.)

use rand::{thread_rng, Rng};
use crate::stats::{Stat, Stats};
use crate::world::types::DamageType;
use crate::world::{SessionId, World, WorldEvent, WorldProvider};

impl<P: WorldProvider> World<P> {
    /// Calculate elemental damage from attacker's equipment, skills, and stats.
    /// Returns a vector of (element_type, damage) pairs.
    /// 
    /// This function collects elemental damage from:
    /// 1. Equipment StatsMap (custom stat IDs for element damage)
    /// 2. Skills/buffs that add element damage (if any)
    /// 3. Stat-based element damage (if Stat enum is extended)
    /// 
    /// Note: Element damage may be stored in StatsMap using custom stat IDs.
    /// Common mappings (if used by C# server):
    /// - FireAttack = 60, IceAttack = 61, LightningAttack = 62, etc.
    /// This is a framework implementation that can be extended when exact stat IDs are known.
    pub fn calculate_elemental_damage(
        &self,
        attacker_session_id: SessionId,
    ) -> Vec<(DamageType, i32)> {
        let mut elemental_damages = Vec::new();
        
        let player = match self.players.get(&attacker_session_id) {
            Some(p) => p,
            None => return elemental_damages,
        };

        // Element damage stat ID mappings (custom stat IDs, not in standard Stat enum)
        // These are placeholder values - actual IDs need to be confirmed from C# server
        const FIRE_ATTACK_STAT_ID: u8 = 60;
        const ICE_ATTACK_STAT_ID: u8 = 61;
        const LIGHTNING_ATTACK_STAT_ID: u8 = 62;
        const WIND_ATTACK_STAT_ID: u8 = 63;
        const EARTH_ATTACK_STAT_ID: u8 = 64;
        const HOLY_ATTACK_STAT_ID: u8 = 65;
        const DARK_ATTACK_STAT_ID: u8 = 66;

        // Collect element damage from all equipped items
        let mut fire_damage = 0;
        let mut ice_damage = 0;
        let mut lightning_damage = 0;
        let mut wind_damage = 0;
        let mut earth_damage = 0;
        let mut holy_damage = 0;
        let mut dark_damage = 0;

        for slot_opt in &player.equipment.slots {
            if let Some(item) = slot_opt {
                // Check base item stats
                if let Some(item_info) = self.provider.get_item_info(item.item_index) {
                    for (stat_id, value) in &item_info.stats.entries {
                        match *stat_id {
                            FIRE_ATTACK_STAT_ID => fire_damage += value,
                            ICE_ATTACK_STAT_ID => ice_damage += value,
                            LIGHTNING_ATTACK_STAT_ID => lightning_damage += value,
                            WIND_ATTACK_STAT_ID => wind_damage += value,
                            EARTH_ATTACK_STAT_ID => earth_damage += value,
                            HOLY_ATTACK_STAT_ID => holy_damage += value,
                            DARK_ATTACK_STAT_ID => dark_damage += value,
                            _ => {}
                        }
                    }
                }

                // Check added stats (from refinement, awakening, etc.)
                for (stat_id, value) in &item.added_stats.entries {
                    match *stat_id {
                        FIRE_ATTACK_STAT_ID => fire_damage += value,
                        ICE_ATTACK_STAT_ID => ice_damage += value,
                        LIGHTNING_ATTACK_STAT_ID => lightning_damage += value,
                        WIND_ATTACK_STAT_ID => wind_damage += value,
                        EARTH_ATTACK_STAT_ID => earth_damage += value,
                        HOLY_ATTACK_STAT_ID => holy_damage += value,
                        DARK_ATTACK_STAT_ID => dark_damage += value,
                        _ => {}
                    }
                }
            }
        }

        // Add element damage from buffs (if any buffs provide element damage)
        // Note: Buff system uses Stats (BTreeMap<Stat, i32>) which only supports
        // standard Stat enum values, not custom stat IDs like element damage.
        // If C# server has buffs that provide element damage, they would need to
        // be implemented via a different mechanism (e.g., custom buff values or
        // extended StatsMap support in buffs).
        // For now, element damage is only collected from equipment.

        // Build result vector (only include non-zero damages)
        if fire_damage > 0 {
            elemental_damages.push((DamageType::Fire, fire_damage));
        }
        if ice_damage > 0 {
            elemental_damages.push((DamageType::Ice, ice_damage));
        }
        if lightning_damage > 0 {
            elemental_damages.push((DamageType::Lightning, lightning_damage));
        }
        if wind_damage > 0 {
            elemental_damages.push((DamageType::Wind, wind_damage));
        }
        if earth_damage > 0 {
            elemental_damages.push((DamageType::Earth, earth_damage));
        }
        if holy_damage > 0 {
            elemental_damages.push((DamageType::Holy, holy_damage));
        }
        if dark_damage > 0 {
            elemental_damages.push((DamageType::Dark, dark_damage));
        }

        elemental_damages
    }

    /// Apply elemental damage to target, considering elemental resistance
    /// Returns the total elemental damage applied
    /// 
    /// This function processes each element type separately:
    /// 1. Gets elemental resistance for the element type
    /// 2. Calculates MAC-based armour (elemental damage uses MAC)
    /// 3. Applies resistance reduction (percentage-based)
    /// 4. Applies armour reduction
    /// 5. Applies the final elemental damage
    pub fn apply_elemental_damage(
        &mut self,
        attacker_session_id: Option<SessionId>,
        target_session_id: SessionId,
        elemental_damages: &[(DamageType, i32)],
        map_index: i32,
        events: &mut Vec<WorldEvent>,
    ) -> i32 {
        if elemental_damages.is_empty() {
            return 0;
        }

        let defender_stats = match self.players.get(&target_session_id) {
            Some(p) => p.stats.total.clone(),
            None => return 0,
        };

        let mut total_elemental_damage = 0;
        let mut rng = thread_rng();

        for (element_type, base_damage) in elemental_damages {
            if *base_damage <= 0 {
                continue;
            }

            // Get elemental resistance for this element type
            let resistance_stat = element_type.get_resistance_stat();
            let resistance = defender_stats.get(resistance_stat).max(0);

            // Calculate elemental armour (uses MAC for elemental damage)
            let min_mac = defender_stats.get(Stat::MinMAC).max(0);
            let max_mac = defender_stats.get(Stat::MaxMAC).max(min_mac);
            let elemental_armour = if max_mac <= min_mac {
                min_mac
            } else {
                rng.gen_range(min_mac..=max_mac)
            };

            // Apply resistance reduction (resistance is a percentage)
            // Formula: damage = base_damage * (100 - resistance) / 100
            let mut element_damage = *base_damage;
            if resistance > 0 {
                element_damage = (element_damage * (100 - resistance.clamp(0, 100))) / 100;
            }

            // Apply armour reduction
            element_damage = (element_damage - elemental_armour).max(0);

            if element_damage > 0 {
                total_elemental_damage += element_damage;
                // Elemental damage indicator is sent via apply_damage_to_player
                // which creates ObjectStruck event with the correct DamageType (Fire, Ice, etc.)
                // The client will display the appropriate damage indicator based on damage_type
            }
        }

        // Apply total elemental damage to target
        if total_elemental_damage > 0 {
            // Apply elemental damage using the first element type (or Physical as fallback)
            let primary_element = elemental_damages[0].0;
            // Note: apply_damage_to_player will be available after damage module is created
            // For now, we'll use the method directly on World
            let _ = self.apply_damage_to_player(
                attacker_session_id,
                target_session_id,
                total_elemental_damage,
                primary_element,
                map_index,
                events,
            );
        }

        total_elemental_damage
    }
}


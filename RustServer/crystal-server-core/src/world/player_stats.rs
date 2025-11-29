use crate::stats::{Stat, Stats};
use super::{configs::base_stats, Job};

#[derive(Clone, Debug, Default)]
pub struct PlayerStats {
    pub base: Stats,
    pub equip: Stats,
    pub buffs: Stats,
    pub passives: Stats,
    pub env: Stats,
    pub total: Stats,
    pub dirty_mask: u64,
}

#[derive(Copy, Clone, Debug)]
pub enum AttributeGroup {
    BasePrimary,
    Offense,
    Defense,
    Resist,
    Move,
    Misc,
}

impl AttributeGroup {
    pub fn mask(self) -> u64 {
        match self {
            AttributeGroup::BasePrimary => 1 << 0,
            AttributeGroup::Offense => 1 << 1,
            AttributeGroup::Defense => 1 << 2,
            AttributeGroup::Resist => 1 << 3,
            AttributeGroup::Move => 1 << 4,
            AttributeGroup::Misc => 1 << 5,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum RecalcReason {
    LevelUp,
    EquipChanged,
    BuffChanged,
    PassiveChanged,
    EnvChanged,
    Login,
}

impl RecalcReason {
    pub fn groups(self) -> u64 {
        match self {
            RecalcReason::LevelUp => {
                AttributeGroup::BasePrimary.mask()
                    | AttributeGroup::Offense.mask()
                    | AttributeGroup::Defense.mask()
                    | AttributeGroup::Resist.mask()
                    | AttributeGroup::Move.mask()
                    | AttributeGroup::Misc.mask()
            }
            RecalcReason::EquipChanged => {
                AttributeGroup::Offense.mask()
                    | AttributeGroup::Defense.mask()
                    | AttributeGroup::Resist.mask()
                    | AttributeGroup::Move.mask()
            }
            RecalcReason::BuffChanged => {
                AttributeGroup::Offense.mask()
                    | AttributeGroup::Defense.mask()
                    | AttributeGroup::Resist.mask()
                    | AttributeGroup::Move.mask()
                    | AttributeGroup::Misc.mask()
            }
            RecalcReason::PassiveChanged => {
                AttributeGroup::Offense.mask()
                    | AttributeGroup::Defense.mask()
                    | AttributeGroup::Resist.mask()
                    | AttributeGroup::Misc.mask()
            }
            RecalcReason::EnvChanged => {
                AttributeGroup::Offense.mask()
                    | AttributeGroup::Defense.mask()
                    | AttributeGroup::Resist.mask()
                    | AttributeGroup::Move.mask()
                    | AttributeGroup::Misc.mask()
            }
            RecalcReason::Login => {
                AttributeGroup::BasePrimary.mask()
                    | AttributeGroup::Offense.mask()
                    | AttributeGroup::Defense.mask()
                    | AttributeGroup::Resist.mask()
                    | AttributeGroup::Move.mask()
                    | AttributeGroup::Misc.mask()
            }
        }
    }
}

impl PlayerStats {
    pub fn mark_dirty(&mut self, reason: RecalcReason) {
        self.dirty_mask |= reason.groups();
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_mask = 0;
    }

    pub fn set_base_from_level(&mut self, job: Job, level: u16) {
        self.base = base_stats::calc_base_stats_for_level(job, level);
        self.dirty_mask |= AttributeGroup::BasePrimary.mask();
    }

    /// Replace the aggregated equipment stats layer with the provided values.
    /// Callers are expected to build `equip_stats` by summing all equipped
    /// items in a way that mirrors C# RefreshEquipmentStats, including base
    /// item stats, added stats, awakening bonuses, sockets, and set bonuses.
    pub fn set_equip_stats(&mut self, equip_stats: &Stats) {
        self.equip = equip_stats.clone();
        self.mark_dirty(RecalcReason::EquipChanged);
    }

    /// Replace the aggregated buff stats layer with the provided values.
    /// Callers are expected to build `buff_stats` by summing all active buffs
    /// in a way that mirrors C# RefreshBuffs (Buff.Stats plus any special
    /// handling for transform, fast run, etc.).
    pub fn set_buff_stats(&mut self, buff_stats: &Stats) {
        self.buffs = buff_stats.clone();
        self.mark_dirty(RecalcReason::BuffChanged);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty_mask != 0
    }

    pub fn recalc_if_dirty(&mut self) {
        if self.dirty_mask != 0 {
            self.recalc_all();
            self.dirty_mask = 0;
        }
    }

    pub fn recalc_if_dirty_for_job(&mut self, job: Job) {
        if self.dirty_mask != 0 {
            self.recalc_all_for_job(job);
            self.dirty_mask = 0;
        }
    }

    fn recalc_all(&mut self) {
        self.total.clear();
        self.total.add(&self.base);
        self.total.add(&self.equip);
        self.total.add(&self.buffs);
        self.total.add(&self.passives);
        self.total.add(&self.env);
    }

    fn recalc_all_for_job(&mut self, job: Job) {
        self.recalc_all();
        self.apply_caps_and_clamps(job);
    }

    fn apply_caps_and_clamps(&mut self, job: Job) {
        // Apply rate-percent modifiers in the same way as C# HumanObject.RefreshStats,
        // so that derived caps such as HP/MP and Max* correctly reflect any
        // HPRatePercent/MPRatePercent and related stats supplied by equipment or
        // buffs.
        let base_hp = self.total.get(Stat::HP);
        let hp_rate = self.total.get(Stat::HPRatePercent);
        if base_hp != 0 && hp_rate != 0 {
            let delta = (base_hp * hp_rate) / 100;
            self.total.set(Stat::HP, base_hp.saturating_add(delta));
        }

        let base_mp = self.total.get(Stat::MP);
        let mp_rate = self.total.get(Stat::MPRatePercent);
        if base_mp != 0 && mp_rate != 0 {
            let delta = (base_mp * mp_rate) / 100;
            self.total.set(Stat::MP, base_mp.saturating_add(delta));
        }

        let base_max_ac = self.total.get(Stat::MaxAC);
        let max_ac_rate = self.total.get(Stat::MaxACRatePercent);
        if base_max_ac != 0 && max_ac_rate != 0 {
            let delta = (base_max_ac * max_ac_rate) / 100;
            self.total
                .set(Stat::MaxAC, base_max_ac.saturating_add(delta));
        }

        let base_max_mac = self.total.get(Stat::MaxMAC);
        let max_mac_rate = self.total.get(Stat::MaxMACRatePercent);
        if base_max_mac != 0 && max_mac_rate != 0 {
            let delta = (base_max_mac * max_mac_rate) / 100;
            self.total
                .set(Stat::MaxMAC, base_max_mac.saturating_add(delta));
        }

        let base_max_dc = self.total.get(Stat::MaxDC);
        let max_dc_rate = self.total.get(Stat::MaxDCRatePercent);
        if base_max_dc != 0 && max_dc_rate != 0 {
            let delta = (base_max_dc * max_dc_rate) / 100;
            self.total
                .set(Stat::MaxDC, base_max_dc.saturating_add(delta));
        }

        let base_max_mc = self.total.get(Stat::MaxMC);
        let max_mc_rate = self.total.get(Stat::MaxMCRatePercent);
        if base_max_mc != 0 && max_mc_rate != 0 {
            let delta = (base_max_mc * max_mc_rate) / 100;
            self.total
                .set(Stat::MaxMC, base_max_mc.saturating_add(delta));
        }

        let base_max_sc = self.total.get(Stat::MaxSC);
        let max_sc_rate = self.total.get(Stat::MaxSCRatePercent);
        if base_max_sc != 0 && max_sc_rate != 0 {
            let delta = (base_max_sc * max_sc_rate) / 100;
            self.total
                .set(Stat::MaxSC, base_max_sc.saturating_add(delta));
        }

        let base_as = self.total.get(Stat::AttackSpeed);
        let as_rate = self.total.get(Stat::AttackSpeedRatePercent);
        if base_as != 0 && as_rate != 0 {
            let delta = (base_as * as_rate) / 100;
            self.total
                .set(Stat::AttackSpeed, base_as.saturating_add(delta));
        }

        // Apply caps for certain stats (e.g. resistances, recovery) based on
        // job-specific configuration.
        let caps = base_stats::base_caps_for_job(job);
        for (stat, cap_val) in &caps.values {
            let current = self.total.get(*stat);
            if current > *cap_val {
                self.total.set(*stat, *cap_val);
            }
        }

        let hp = self.total.get(Stat::HP).max(0);
        let mp = self.total.get(Stat::MP).max(0);
        self.total.set(Stat::HP, hp);
        self.total.set(Stat::MP, mp);

        let min_ac = self.total.get(Stat::MinAC).max(0);
        let max_ac = self.total.get(Stat::MaxAC).max(0);
        let min_mac = self.total.get(Stat::MinMAC).max(0);
        let max_mac = self.total.get(Stat::MaxMAC).max(0);
        let min_dc = self.total.get(Stat::MinDC).max(0);
        let max_dc = self.total.get(Stat::MaxDC).max(0);
        let min_mc = self.total.get(Stat::MinMC).max(0);
        let max_mc = self.total.get(Stat::MaxMC).max(0);
        let min_sc = self.total.get(Stat::MinSC).max(0);
        let max_sc = self.total.get(Stat::MaxSC).max(0);

        self.total.set(Stat::MinAC, min_ac);
        self.total.set(Stat::MaxAC, max_ac);
        self.total.set(Stat::MinMAC, min_mac);
        self.total.set(Stat::MaxMAC, max_mac);
        self.total.set(Stat::MaxDC, max_dc);
        self.total.set(Stat::MaxMC, max_mc);
        self.total.set(Stat::MaxSC, max_sc);

        self.total.set(Stat::MinDC, min_dc.min(max_dc));
        self.total.set(Stat::MinMC, min_mc.min(max_mc));
        self.total.set(Stat::MinSC, min_sc.min(max_sc));
    }
}

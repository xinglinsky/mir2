use crate::stats::Stats;

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

    pub fn is_dirty(&self) -> bool {
        self.dirty_mask != 0
    }

    pub fn recalc_if_dirty(&mut self) {
        if self.dirty_mask != 0 {
            self.recalc_all();
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
}

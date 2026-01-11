use crystal_shared_proto::hero::{
    CChangeHero,
    CNewHero,
    CSetAutoPotItem,
    CSetAutoPotValue,
    CSetHeroBehaviour,
    CTakeBackHeroItem,
    CTransferHeroItem,
    SChangeHero,
    SHeroCreateRequest,
    SManageHeroes,
    SNewHero,
    SSetAutoPotItem,
    SSetAutoPotValue,
    SSetHeroBehaviour,
};
use crystal_shared_proto::item::{STakeBackHeroItem, STransferHeroItem};
use crystal_shared_proto::io::{write_bool, write_i32_le, write_string, write_u16_le};

use crystal_server_core::account::{StoredHeroState, StoredHeroSummary};

use super::{HeroSummary, LoginConnection, Stage};

impl LoginConnection {
    pub(crate) fn save_hero_state_to_store(&self) {
        let (Some(account_id), Some(char_idx)) = (self.account_id.as_ref(), self.current_char_index) else {
            return;
        };

        let heroes: Vec<Option<StoredHeroSummary>> = self
            .hero_storage
            .iter()
            .take(8)
            .map(|h| {
                h.as_ref().map(|hh| StoredHeroSummary {
                    index: hh.index,
                    name: hh.name.clone(),
                    level: hh.level,
                    class: hh.class,
                    gender: hh.gender,
                })
            })
            .collect();

        let current_hero_index = self.hero_current.as_ref().map(|h| h.index).unwrap_or(0);
        let hero_spawned = self.hero_spawn_state >= 2;

        let state = StoredHeroState {
            maximum_count: self.hero_maximum_count,
            next_index: self.hero_next_index,
            current_hero_index,
            hero_spawned,
            hero_behaviour: self.hero_behaviour,
            heroes,
        };

        let _ = self.store.save_character_hero_state(account_id, char_idx, &state);
    }

    fn encode_client_hero_information(info: &HeroSummary) -> std::io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        write_i32_le(&mut buf, info.index)?;
        write_string(&mut buf, &info.name)?;
        write_u16_le(&mut buf, info.level)?;
        buf.push(info.class);
        buf.push(info.gender);
        Ok(buf)
    }

    pub(crate) fn send_manage_heroes(&mut self, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let mut heroes_bytes = Vec::new();

        let current = self.hero_current.clone();
        write_bool(&mut heroes_bytes, current.is_some()).ok();
        if let Some(ch) = &current {
            if let Ok(b) = Self::encode_client_hero_information(ch) {
                heroes_bytes.extend_from_slice(&b);
            }
        }

        write_bool(&mut heroes_bytes, true).ok();
        write_i32_le(&mut heroes_bytes, 8).ok();
        for i in 0..8 {
            let entry = self.hero_storage.get(i).and_then(|e| e.as_ref());
            write_bool(&mut heroes_bytes, entry.is_some()).ok();
            if let Some(h) = entry {
                if let Ok(b) = Self::encode_client_hero_information(h) {
                    heroes_bytes.extend_from_slice(&b);
                }
            }
        }

        let pkt = SManageHeroes {
            maximum_count: self.hero_maximum_count,
            heroes_bytes,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn send_hero_create_request(&mut self, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let pkt = SHeroCreateRequest {
            can_create_class: vec![true, true, true, true, true],
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
}

impl LoginConnection {
    pub(crate) fn handle_new_hero(&mut self, _msg: CNewHero, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        if self.hero_storage.is_empty() {
            self.hero_storage = vec![None; 8];
        }

        if self.hero_storage.iter().filter(|h| h.is_some()).count() >= 8 {
            let pkt = SNewHero { result: 4 };
            out.push(Self::encode_raw(pkt.encode()));
            return;
        }

        let msg = _msg;
        let hero = HeroSummary {
            index: self.hero_next_index,
            name: msg.name,
            level: 1,
            class: msg.class,
            gender: msg.gender,
        };
        self.hero_next_index = self.hero_next_index.saturating_add(1);

        if self.hero_current.is_none() {
            self.hero_current = Some(hero.clone());
        }

        if let Some(slot) = self.hero_storage.iter().position(|h| h.is_none()) {
            self.hero_storage[slot] = Some(hero);
        }

        let pkt = SNewHero { result: 10 };
        out.push(Self::encode_raw(pkt.encode()));
        self.send_manage_heroes(out);

        self.save_hero_state_to_store();
    }

    pub(crate) fn handle_set_hero_behaviour(&mut self, msg: CSetHeroBehaviour, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        self.hero_behaviour = msg.behaviour;

        let pkt = SSetHeroBehaviour {
            behaviour: msg.behaviour,
        };
        out.push(Self::encode_raw(pkt.encode()));

        self.save_hero_state_to_store();
    }

    pub(crate) fn handle_change_hero(&mut self, msg: CChangeHero, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let mut from_index = msg.list_index - 1;
        if from_index < 0 {
            from_index = 0;
        }

        if let Some(sel) = self.hero_storage.get(from_index as usize).and_then(|h| h.clone()) {
            self.hero_current = Some(sel);
        }

        let pkt = SChangeHero { from_index };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }

        if self.hero_spawn_state >= 2 {
            self.send_hero_bootstrap(out);
        }

        self.save_hero_state_to_store();
    }

    pub(crate) fn handle_set_auto_pot_value(&mut self, msg: CSetAutoPotValue, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let pkt = SSetAutoPotValue {
            stat: msg.stat,
            value: msg.value,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_set_auto_pot_item(&mut self, msg: CSetAutoPotItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let pkt = SSetAutoPotItem {
            grid: msg.grid,
            item_index: msg.item_index,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_take_back_hero_item(&mut self, msg: CTakeBackHeroItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let pkt = STakeBackHeroItem {
            from: msg.from,
            to: msg.to,
            success: false,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }

    pub(crate) fn handle_transfer_hero_item(&mut self, msg: CTransferHeroItem, out: &mut Vec<Vec<u8>>) {
        if self.stage != Stage::InGame {
            return;
        }

        let pkt = STransferHeroItem {
            from: msg.from,
            to: msg.to,
            success: false,
        };
        if let Ok(raw) = pkt.encode() {
            out.push(Self::encode_raw(raw));
        }
    }
}

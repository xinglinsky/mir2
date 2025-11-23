use crystal_server_core::world::{self, WorldProvider};
use crystal_shared_proto::scene::{SObjectRemove, SLevelChanged, SObjectLeveled};
use crystal_shared_proto::user::{SGainedGold, SHealthChanged};

use super::LoginConnection;

impl LoginConnection {
    /// Handle all GM/admin chat commands.
    /// Returns true if the command was handled and chat processing should stop.
    pub(crate) fn handle_gm_chat(&mut self, trimmed: &str, out: &mut Vec<Vec<u8>>) -> bool {
        // /gm toggle
        if let Some(rest) = trimmed.strip_prefix("/gm") {
            let arg = rest.trim();
            if arg.eq_ignore_ascii_case("off") {
                self.is_gm = false;
                self.send_system_chat("GM mode disabled.", out);
            } else {
                self.is_gm = true;
                self.send_system_chat("GM mode enabled.", out);
            }
            return true;
        }

        // /createguild
        if let Some(rest) = trimmed.strip_prefix("/createguild ") {
            self.handle_create_guild_command(rest, out);
            return true;
        }

        // /showmemoney
        if let Some(rest) = trimmed.strip_prefix("/showmemoney ") {
            if !self.is_gm {
                self.send_system_chat("GM only command.", out);
                return true;
            }
            if let Ok(delta) = rest.trim().parse::<i64>() {
                if delta > 0 {
                    if let Some(mut stats) = self.current_stats.clone() {
                        let new_gold = stats.gold.saturating_add(delta);
                        stats.gold = new_gold;
                        if let (Some(ref account_id), Some(char_idx)) =
                            (self.account_id.as_ref(), self.current_char_index)
                        {
                            let _ = self
                                .store
                                .save_character_stats(account_id, char_idx, &stats);
                        }
                        self.current_stats = Some(stats.clone());

                        let gained = SGainedGold {
                            gold: delta as u32,
                        };
                        if let Ok(raw) = gained.encode() {
                            out.push(LoginConnection::encode_raw(raw));
                        }
                    }
                }
            }
            return true;
        }

        // /level
        if let Some(rest) = trimmed.strip_prefix("/level ") {
            if !self.is_gm {
                self.send_system_chat("GM only command.", out);
                return true;
            }

            let arg = rest.trim();
            if arg.is_empty() {
                self.send_system_chat("Usage: /level <level>", out);
                return true;
            }

            let Ok(mut level) = arg.parse::<u16>() else {
                self.send_system_chat("Invalid level.", out);
                return true;
            };

            if level == 0 {
                self.send_system_chat("Level must be >= 1.", out);
                return true;
            }

            // Clamp to a reasonable maximum; the C# server uses ushort for
            // Level so we mirror that here.
            if level == u16::MAX {
                level = u16::MAX.saturating_sub(1);
            }

            // Determine an experience value consistent with this level using
            // the same exp_table convention as movement.rs. For simplicity we
            // set experience to 0 within the new level band.
            let exp: i64 = 0;

            // Update world-side player level/experience and recalc stats.
            {
                let mut world = self.world.lock().unwrap();
                if world
                    .set_player_level_and_experience(self.session_id, level, exp)
                    .is_none()
                {
                    self.send_system_chat("/level failed: player not in world.", out);
                    return true;
                }
            }

            // Fetch new max HP/MP from world so we can sync current_stats and
            // send SHealthChanged.
            let (max_hp, max_mp) = {
                let world = self.world.lock().unwrap();
                world
                    .player_max_hp_mp(self.session_id)
                    .unwrap_or((0, 0))
            };

            if let Some(mut stats) = self.current_stats.clone() {
                stats.hp = max_hp.max(0);
                stats.mp = max_mp.max(0);
                stats.experience = exp;

                if let (Some(ref account_id), Some(char_idx)) =
                    (self.account_id.as_ref(), self.current_char_index)
                {
                    let _ = self
                        .store
                        .save_character_stats(account_id, char_idx, &stats);
                }

                self.current_stats = Some(stats.clone());

                // Send HP/MP update.
                let hpmp_pkt = SHealthChanged {
                    hp: stats.hp,
                    mp: stats.mp,
                };
                if let Ok(raw) = hpmp_pkt.encode() {
                    out.push(LoginConnection::encode_raw(raw));
                }
            }

            // Update cached SelectInfo level so that character lists reflect
            // the new level.
            if let Some(char_idx) = self.current_char_index {
                if let Some(ch) = self.characters.iter_mut().find(|c| c.index == char_idx) {
                    ch.level = level;
                }
            }

            // Compute MaxExperience for the new level.
            let max_experience = if level == 0 {
                0_i64
            } else {
                *self
                    .exp_table
                    .get(level as usize - 1)
                    .unwrap_or(&0_i64)
            };

            // Notify client of level/exp band.
            let lvl_pkt = SLevelChanged {
                level,
                experience: exp,
                max_experience,
            };
            if let Ok(raw) = lvl_pkt.encode() {
                out.push(LoginConnection::encode_raw(raw));
            }

            // Broadcast ObjectLeveled to self and nearby observers.
            let obj_pkt = SObjectLeveled {
                object_id: self.session_id,
            };
            if let Ok(pkt) = obj_pkt.encode() {
                let raw = LoginConnection::encode_raw(pkt);
                out.push(raw.clone());
                self.enqueue_for_viewers(
                    self.current_map_index,
                    self.current_x,
                    self.current_y,
                    raw,
                );
            }

            return true;
        }

        // /skill
        if let Some(rest) = trimmed.strip_prefix("/skill") {
            if !self.is_gm {
                self.send_system_chat("GM only command.", out);
                return true;
            }

            // Derive the current character class (0=Warrior,1=Wizard,2=Taoist,3=Assassin,4=Archer)
            let class_id: u8 = if let Some(char_idx) = self.current_char_index {
                self.characters
                    .iter()
                    .find(|c| c.index == char_idx)
                    .map(|c| c.class)
                    .unwrap_or(0)
            } else {
                0
            };

            let is_spell_for_class = |spell: u8, class: u8| -> bool {
                match class {
                    // Warrior spells: 1..=17
                    0 => (1..=17).contains(&spell),
                    // Wizard spells: 31..=55
                    1 => (31..=55).contains(&spell),
                    // Taoist spells: 61..=86
                    2 => (61..=86).contains(&spell),
                    // Assassin spells: 91..=107
                    3 => (91..=107).contains(&spell),
                    // Archer spells: 121..=141
                    4 => (121..=141).contains(&spell),
                    _ => false,
                }
            };

            let arg = rest.trim();
            let mut learned_count: usize = 0;

            if arg.is_empty() {
                // No argument: learn all spells for this class from MagicInfoList.
                let spells: Vec<u8> = self
                    .world_db
                    .magic_infos()
                    .iter()
                    .map(|mi| mi.spell)
                    .filter(|&s| s != 0 && is_spell_for_class(s, class_id))
                    .collect();

                let mut world = self.world.lock().unwrap();
                for spell in spells {
                    if let Some(magic) = world.learn_magic_for_player(self.session_id, spell) {
                        learned_count += 1;
                        self.send_new_magic(&magic, out);
                    }
                }
            } else {
                // With argument: learn a specific spell by id or name.
                let spell_id_opt: Option<u8> = arg.parse::<u8>().ok().or_else(|| {
                    // Fallback: look up by MagicInfo name (case-insensitive).
                    self.world_db
                        .magic_infos()
                        .iter()
                        .find(|mi| mi.name.eq_ignore_ascii_case(arg))
                        .map(|mi| mi.spell)
                });

                let Some(spell_id) = spell_id_opt else {
                    self.send_system_chat("Unknown skill.", out);
                    return true;
                };

                if !is_spell_for_class(spell_id, class_id) {
                    self.send_system_chat("Skill does not belong to your class.", out);
                    return true;
                }

                let magic_opt = {
                    let mut world = self.world.lock().unwrap();
                    world.learn_magic_for_player(self.session_id, spell_id)
                };

                if let Some(magic) = magic_opt {
                    learned_count = 1;
                    self.send_new_magic(&magic, out);
                } else {
                    self.send_system_chat("Skill already learned.", out);
                    return true;
                }
            }

            // Persist the updated magic list for this character.
            if learned_count > 0 {
                if let (Some(ref account_id), Some(char_idx)) =
                    (self.account_id.as_ref(), self.current_char_index)
                {
                    let magics = {
                        let world = self.world.lock().unwrap();
                        world.player_magics(self.session_id)
                    };
                    let _ = self
                        .store
                        .save_character_magics(account_id, char_idx, &magics);
                }

            self.send_system_chat(
                    &format!("Learned {} skill(s).", learned_count),
                    out,
                );
            } else {
                self.send_system_chat("No new skills learned.", out);
            }

            return true;
        }

        // /kill
        if trimmed.eq_ignore_ascii_case("/kill") {
            if !self.is_gm {
                self.send_system_chat("GM only command.", out);
                return true;
            }
            let killed = {
                let mut world = self.world.lock().unwrap();
                world.kill_nearest_monster(
                    self.current_map_index,
                    self.current_x,
                    self.current_y,
                )
            };

            if let Some((id, monster_exp)) = killed {
                self.known_monsters.remove(&id);
                let pkt = SObjectRemove {
                    object_id: id as u32,
                };
                if let Ok(raw) = pkt.encode() {
                    out.push(LoginConnection::encode_raw(raw));
                }

                if monster_exp > 0 {
                    let events = vec![world::WorldEvent::GainExperience {
                        session_id: self.session_id,
                        amount: monster_exp,
                    }];
                    let _ = self.handle_world_events(events, out);
                }
            }

            return true;
        }

        false
    }
}

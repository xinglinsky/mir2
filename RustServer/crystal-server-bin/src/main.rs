use std::{
    fs,
    io,
    net::SocketAddr,
    sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}},
    thread,
    time::{Duration, Instant},
};
use std::collections::HashMap;

use crate::connection::{LoginConnection, PlayerVisual};

use crystal_server_net::{run_server, ConnectionHandler, HandlerFactory};
use crystal_server_core::world::{self, WorldConfig, WorldDatabase, WorldProvider, configs};
use crystal_server_core::account::AccountStore;
use crystal_server_db::AsyncAccountStore;

use crystal_shared_proto::scene::{
    SChat,
    SDeath,
    SObjectAttack,
    SObjectDied,
    SObjectGold,
    SObjectRemove,
    SStruck,
};
use crystal_shared_proto::magic::SObjectEffect;
use crystal_shared_proto::user::SHealthChanged;
use serde::{Deserialize, Serialize};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod config;
mod connection;
mod admin_config;
mod logging;

#[derive(Serialize)]
struct AdminMetrics {
    players: u32,
    monsters: u32,
    connections: u32,
    blocked_ips: u32,
    uptime_seconds: u64,
    cycle_delay_ms: u32,
}

#[derive(Serialize)]
struct AdminPlayerInfo {
    id: u32,
    name: String,
    level: u32,
    class: String,
    gender: String,
    map: String,
}

#[derive(Deserialize)]
struct AdminBroadcast {
    message: String,
}

#[derive(Deserialize)]
struct AdminWorldSettings {
    spawn_multiplier: u16,
    respawn_base_spawn_rate_minutes: u8,
    drop_rate: f32,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cfg = config::load_server_config("server.toml")
        .expect("failed to load server configuration");

    let admin_cfg = match admin_config::load_admin_config("admin.toml") {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!("failed to load admin.toml, admin console integration disabled: {}", e);
            None
        }
    };

    let filter = EnvFilter::new(
        cfg.log_filter
            .as_deref()
            .unwrap_or("info"),
    );

    let shared_logs = logging::SharedLogs::new();
    let log_layer = logging::LogBufferLayer::new(shared_logs.clone());
    let fmt_layer = tracing_subscriber::fmt::layer();

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(log_layer)
        .init();

    let addr: SocketAddr = cfg.listen_addr;
    let timeout_ms = cfg.timeout_ms;
    // Use an asynchronous account store that batches writes and flushes them
    // periodically to reduce latency on the connection threads. Keep the
    // interval relatively small so that logout/character-switch flows see
    // the latest position/stats almost immediately on the next StartGame.
    let flush_interval = Duration::from_millis(500);
    let store: Arc<dyn AccountStore> = Arc::new(
        AsyncAccountStore::open(&cfg.accounts_db_path, flush_interval)
            .expect("failed to open accounts sqlite database"),
    );

    // Load MapInfoList from the C# Server.MirDB database so we can use
    // real map metadata to drive map loading and packets. This keeps
    // compatibility with the existing C# server's database format.
    let mut world_db = WorldDatabase::new();
    match world::map::load_map_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(maps) => {
            tracing::debug!(
                "[core] Loaded {} MapInfo entries from Server.MirDB",
                maps.len()
            );
            world_db.map_infos = maps;
        }
        Err(e) => {
            tracing::debug!(
                "[core] Failed to load Server.MirDB (MapInfoList): {} (continuing with empty DB)",
                e
            );
        }
    }

    // Load GameShopList so that the game shop can use the same data as the
    // original C# server's Envir.GameShopList, keeping the mir.db format as
    // the single source of truth for shop entries.
    match world::map::load_game_shop_items_from_mirdb(&cfg.server_mirdb_path) {
        Ok(shop_items) => {
            tracing::debug!(
                "[core] Loaded {} GameShopItem entries from Server.MirDB",
                shop_items.len()
            );
            world_db.game_shop_items = shop_items;
        }
        Err(e) => {
            tracing::debug!(
                "[core] Failed to load Server.MirDB (GameShopList): {} (continuing without GameShop DB)",
                e
            );
        }
    }

    // Load ItemInfoList so that systems like NPC shops can construct
    // concrete UserItemData payloads for goods, mirroring C# Envir.ItemInfoList.
    match world::map::load_item_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(items) => {
            tracing::debug!(
                "[core] Loaded {} ItemInfo entries from Server.MirDB",
                items.len()
            );
            world_db.item_infos = items;
        }
        Err(e) => {
            tracing::debug!(
                "[core] Failed to load Server.MirDB (ItemInfoList): {} (continuing without Item DB)",
                e
            );
        }
    }

    // Load MonsterInfoList from the same Server.MirDB so combat logic can
    // access real monster definitions. For now this is only stored in
    // WorldDatabase and not yet wired into spawn logic.
    match world::map::load_monster_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(monsters) => {
            tracing::debug!(
                "[core] Loaded {} MonsterInfo entries from Server.MirDB",
                monsters.len()
            );
            world_db.monster_infos = monsters;

            // Load monster drop tables from text files, mirroring the C#
            // behaviour where MonsterInfo.DropPath points to Envir/Drops
            // entries and falls back to the monster Name when DropPath is
            // empty.
            let drops_root = &cfg.drops_path;
            let item_infos = world_db.item_infos.clone();
            let item_lookup = move |name: &str| {
                item_infos
                    .iter()
                    .find(|i| i.name.eq_ignore_ascii_case(name))
                    .map(|item| item.index)
            };

            for m in &mut world_db.monster_infos {
                let file_name = if m.drop_path.is_empty() {
                    if m.name.is_empty() {
                        continue;
                    }
                    format!("{}.txt", m.name)
                } else {
                    format!("{}.txt", m.drop_path)
                };

                let full_path = drops_root.join(&file_name);

                let drops = match world::drop::load_drop_file(&full_path, 0, &item_lookup) {
                    Ok(list) => list,
                    Err(e) => {
                        tracing::debug!(
                            "[core] Failed to load drops for monster {} from {}: {}",
                            m.name,
                            full_path.display(),
                            e
                        );
                        Vec::new()
                    }
                };

                m.drops = drops;
            }
        }
        Err(e) => {
            tracing::debug!(
                "[core] Failed to load Server.MirDB (MonsterInfoList): {} (continuing without Monster DB)",
                e
            );
        }
    }

    // Load NPCInfoList from the same Server.MirDB so world logic can access
    // real NPC definitions. For now this is only stored in WorldDatabase and
    // not yet wired into scene packets.
    match world::map::load_npc_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(npcs) => {
            tracing::debug!(
                "[core] Loaded {} NPCInfo entries from Server.MirDB",
                npcs.len()
            );
            world_db.npc_infos = npcs;
        }
        Err(e) => {
            tracing::debug!(
                "[core] Failed to load Server.MirDB (NPCInfoList): {} (continuing without NPC DB)",
                e
            );
        }
    }

    // Load MagicInfoList so that the combat/skill system can access real spell
    // definitions (costs, ranges, power, etc.) from the MirDB.
    match world::map::load_magic_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(magics) => {
            tracing::debug!(
                "[core] Loaded {} MagicInfo entries from Server.MirDB",
                magics.len()
            );
            world_db.magic_infos = magics;
        }
        Err(e) => {
            tracing::debug!(
                "[core] Failed to load Server.MirDB (MagicInfoList): {} (continuing without Magic DB)",
                e
            );
        }
    }

    let buff_infos = world::buff::load_default_buff_infos(cfg.game_master_effect);
    tracing::debug!("[core] Loaded {} Buffs.", buff_infos.len());
    world_db.buff_infos = buff_infos;

    match world::recipe::load_recipes_from_dir(&cfg.recipes_path, &world_db.item_infos) {
        Ok(recipes) => {
            tracing::debug!(
                "[core] Loaded {} Recipes from {}",
                recipes.len(),
                cfg.recipes_path.display()
            );
            world_db.recipe_infos = recipes;
        }
        Err(e) => {
            tracing::debug!(
                "[core] Failed to load Recipes from {}: {} (continuing without Recipe DB)",
                cfg.recipes_path.display(),
                e
            );
        }
    }
    let world_db = Arc::new(world_db);

    let world_map_setup = configs::world_map_setup().clone();

    // Map directory and spawn settings come from configuration, mirroring
    // C# Settings.MapPath and Envir.SpawnMultiplier / RespawnTick.BaseSpawnRate.
    // SafeZoneBorder/SafeZoneHealing are also configurable to match Setup.ini.
    let world_config = WorldConfig::new(
        &cfg.maps_path,
        &cfg.routes_path,
        cfg.spawn_multiplier,
        cfg.respawn_base_spawn_rate_minutes,
        cfg.drop_rate,
        cfg.teleport_to_npc_cost,
        world_map_setup,
        cfg.safe_zone_border,
        cfg.safe_zone_healing,
    );

    // Global world instance shared by all connections, mirroring the single-world
    // design of the original C# server. After construction, load any persisted
    // guild definitions from the account store so that guilds survive restarts.
    let mut world = world::World::new((*world_db).clone(), world_config.clone());
    if let Ok(guilds) = store.load_all_guilds() {
        world.init_guilds_from_db(guilds);
    }
    let world = Arc::new(Mutex::new(world));

    // Per-session outbound packet queues, shared by connection handlers and
    // the world tick thread that emits monster movement and other events.
    let outboxes: Arc<Mutex<HashMap<world::SessionId, Vec<Vec<u8>>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Track which account_id is currently bound to which session_id so we can
    // mirror the C# behaviour where a second login for the same account
    // disconnects the previous session.
    let online_accounts: Arc<Mutex<HashMap<String, world::SessionId>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Simple world tick thread driving the World::update loop. The tick
    // interval roughly mirrors the C# Envir.Process cadence (50–100ms range)
    // and also emits world events such as monster movement.
    {
        let world_for_tick = Arc::clone(&world);
        let outboxes_for_world_events: Arc<Mutex<HashMap<world::SessionId, Vec<Vec<u8>>>>> =
            Arc::clone(&outboxes);
        thread::spawn(move || {
            let tick = Duration::from_millis(50);
            let start = Instant::now();
            loop {
                let now_ms = start.elapsed().as_millis() as i64;
                let events = {
                    let mut w = world_for_tick.lock().unwrap();
                    w.update(now_ms)
                };

                for event in events {
                    match event {
                        world::WorldEvent::ObjectLocation {
                            object_id,
                            map_index,
                            x,
                            y,
                            direction,
                        } => {
                            let viewers = {
                                let w = world_for_tick.lock().unwrap();
                                w.sessions_in_range_for_map(
                                    map_index,
                                    x,
                                    y,
                                    LoginConnection::DATA_RANGE,
                                )
                            };

                            if viewers.is_empty() {
                                continue;
                            }

                            let base =
                                crystal_shared_proto::scene::SObjectTurnWalkRun {
                                    object_id: object_id as u32,
                                    location_x: x,
                                    location_y: y,
                                    direction,
                                };

                            if let Ok(pkt) =
                                crystal_shared_proto::scene::SObjectWalk(base).encode()
                            {
                                let raw = pkt.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                for sid in viewers {
                                    outboxes.entry(sid).or_default().push(raw.clone());
                                }
                            }
                        }
                        world::WorldEvent::ItemDropped { .. } => {
                            // Item drop scene packets are emitted from the
                            // connection layer (movement.rs) when handling
                            // WorldEvent::ItemDropped for a given session.
                        }
                        world::WorldEvent::GoldDropped {
                            object_id,
                            map_index,
                            x,
                            y,
                            gold,
                        } => {
                            let viewers = {
                                let w = world_for_tick.lock().unwrap();
                                w.sessions_in_range_for_map(
                                    map_index,
                                    x,
                                    y,
                                    LoginConnection::DATA_RANGE,
                                )
                            };

                            tracing::debug!(
                                "[drop-send] GoldDropped: object_id={} map={} pos=({}, {}) gold={} viewers={}",
                                object_id,
                                map_index,
                                x,
                                y,
                                gold,
                                viewers.len(),
                            );

                            if viewers.is_empty() || gold == 0 {
                                continue;
                            }

                            let pkt = SObjectGold {
                                object_id: object_id as u32,
                                gold,
                                location_x: x,
                                location_y: y,
                            };

                            if let Ok(raw) = pkt.encode() {
                                let encoded = raw.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                for sid in &viewers {
                                    outboxes
                                        .entry(*sid)
                                        .or_default()
                                        .push(encoded.clone());
                                }
                            }
                        }
                        world::WorldEvent::MapItemRemoved {
                            object_id,
                            map_index,
                            x,
                            y,
                        } => {
                            let viewers = {
                                let w = world_for_tick.lock().unwrap();
                                w.sessions_in_range_for_map(
                                    map_index,
                                    x,
                                    y,
                                    LoginConnection::DATA_RANGE,
                                )
                            };

                            tracing::trace!(
                                "[tick] MapItemRemoved: object_id={} map={} pos=({}, {}) viewers={}",
                                object_id,
                                map_index,
                                x,
                                y,
                                viewers.len()
                            );

                            if viewers.is_empty() {
                                continue;
                            }

                            let pkt = SObjectRemove {
                                object_id: object_id as u32,
                            };

                            if let Ok(raw) = pkt.encode() {
                                let encoded = raw.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                for sid in &viewers {
                                    outboxes
                                        .entry(*sid)
                                        .or_default()
                                        .push(encoded.clone());
                                }
                            }
                        }
                        world::WorldEvent::ObjectAttack {
                            session_id,
                            map_index,
                            x,
                            y,
                            direction,
                            spell,
                            level,
                            attack_type,
                        } => {
                            let viewers = {
                                let w = world_for_tick.lock().unwrap();
                                w.sessions_in_range_for_map(
                                    map_index,
                                    x,
                                    y,
                                    LoginConnection::DATA_RANGE,
                                )
                            };

                            if viewers.is_empty() {
                                continue;
                            }

                            let pkt = SObjectAttack {
                                object_id: session_id,
                                location_x: x,
                                location_y: y,
                                direction,
                                spell,
                                level,
                                attack_type,
                            };

                            if let Ok(raw) = pkt.encode() {
                                let encoded = raw.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                for sid in &viewers {
                                    outboxes
                                        .entry(*sid)
                                        .or_default()
                                        .push(encoded.clone());
                                }
                            }
                        }
                        world::WorldEvent::ObjectStruck {
                            attacker_id,
                            target_id,
                            map_index,
                            x,
                            y,
                            direction,
                            damage,
                            damage_type,
                            health_percent,
                        } => {
                            // Delayed hits from World::update (pure magic
                            // attacks, pet attacks, etc.) surface here via
                            // ObjectStruck. Forward them to all nearby
                            // sessions as SObjectStruck / SDamageIndicator /
                            // SObjectHealth so the client can render hit
                            // animations, damage numbers and monster HP bars.
                            let viewers = {
                                let w = world_for_tick.lock().unwrap();
                                w.sessions_in_range_for_map(
                                    map_index,
                                    x,
                                    y,
                                    LoginConnection::DATA_RANGE,
                                )
                            };

                            if viewers.is_empty() {
                                continue;
                            }

                            let object_id = target_id as u32;

                            let struck_pkt = crystal_shared_proto::scene::SObjectStruck {
                                object_id,
                                // The client only needs attacker_id as an
                                // object reference; here attacker_id is a
                                // SessionId for players, which matches the
                                // object_id used for player objects on the
                                // client.
                                attacker_id,
                                location_x: x,
                                location_y: y,
                                direction,
                            };
                            if let Ok(pkt) = struck_pkt.encode() {
                                let raw = pkt.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                for sid in &viewers {
                                    outboxes
                                        .entry(*sid)
                                        .or_default()
                                        .push(raw.clone());
                                }
                            }

                            let dmg_pkt = crystal_shared_proto::scene::SDamageIndicator {
                                damage,
                                damage_type,
                                object_id,
                            };
                            if let Ok(pkt) = dmg_pkt.encode() {
                                let raw = pkt.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                for sid in &viewers {
                                    outboxes
                                        .entry(*sid)
                                        .or_default()
                                        .push(raw.clone());
                                }
                            }

                            let health_pkt = crystal_shared_proto::scene::SObjectHealth {
                                object_id,
                                percent: health_percent,
                                expire: 5,
                            };
                            if let Ok(pkt) = health_pkt.encode() {
                                let raw = pkt.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                for sid in &viewers {
                                    outboxes
                                        .entry(*sid)
                                        .or_default()
                                        .push(raw.clone());
                                }
                            }
                        }
                        world::WorldEvent::MonsterDied {
                            object_id,
                            map_index,
                            x,
                            y,
                            direction,
                        } => {
                            let viewers = {
                                let w = world_for_tick.lock().unwrap();
                                w.sessions_in_range_for_map(
                                    map_index,
                                    x,
                                    y,
                                    LoginConnection::DATA_RANGE,
                                )
                            };

                            if viewers.is_empty() {
                                continue;
                            }

                            let pkt = SObjectDied {
                                object_id: object_id as u32,
                                location_x: x,
                                location_y: y,
                                direction,
                                death_type: 0,
                            };

                            if let Ok(raw) = pkt.encode() {
                                let encoded = raw.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                for sid in &viewers {
                                    outboxes
                                        .entry(*sid)
                                        .or_default()
                                        .push(encoded.clone());
                                }
                            }
                        }
                        world::WorldEvent::MonsterHitPlayer {
                            attacker_monster_id,
                            session_id,
                            map_index,
                            x,
                            y,
                            direction,
                            damage,
                            damage_type,
                            health_percent,
                        } => {
                            let (mut viewers, hp_mp, attacker_pos) = {
                                let w = world_for_tick.lock().unwrap();
                                let viewers = w.sessions_in_range_for_map(
                                    map_index,
                                    x,
                                    y,
                                    LoginConnection::DATA_RANGE,
                                );
                                let hp_mp = w.player_current_hp_mp(session_id);
                                let attacker_pos =
                                    w.monster_position(map_index, attacker_monster_id);
                                (viewers, hp_mp, attacker_pos)
                            };

                            // Before we remove the struck player from the
                            // broadcast list, capture the full viewer set for
                            // the monster's attack animation.
                            let viewers_for_attack = viewers.clone();

                            // If we can locate the attacking monster's
                            // position and facing, emit an SObjectAttack so
                            // that both the struck player and observers see
                            // the monster swing animation and hear its attack
                            // sound, mirroring the C# server behaviour.
                            if let Some((ax, ay, adir)) = attacker_pos {
                                let atk_pkt = SObjectAttack {
                                    object_id: attacker_monster_id as u32,
                                    location_x: ax,
                                    location_y: ay,
                                    direction: adir,
                                    spell: 0,
                                    level: 0,
                                    attack_type: 0,
                                };
                                if let Ok(pkt) = atk_pkt.encode() {
                                    let raw = pkt.encode();
                                    let mut outboxes =
                                        outboxes_for_world_events.lock().unwrap();
                                    for sid in &viewers_for_attack {
                                        outboxes
                                            .entry(*sid)
                                            .or_default()
                                            .push(raw.clone());
                                    }
                                }
                            }

                            // Ensure the struck player always receives their own
                            // SStruck packet locally for hit animation.
                            let struck = SStruck {
                                attacker_id: attacker_monster_id as u32,
                            };
                            if let Ok(pkt) = struck.encode() {
                                let encoded = pkt.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                outboxes
                                    .entry(session_id)
                                    .or_default()
                                    .push(encoded);
                            }

                            // Broadcast-oriented viewers should not include the
                            // struck player; they already receive dedicated
                            // self-targeted packets.
                            if let Some(pos) = viewers.iter().position(|sid| *sid == session_id) {
                                viewers.swap_remove(pos);
                            }

                            let object_id = session_id;

                            if !viewers.is_empty() {
                                let struck_pkt = crystal_shared_proto::scene::SObjectStruck {
                                    object_id,
                                    attacker_id: attacker_monster_id as u32,
                                    location_x: x,
                                    location_y: y,
                                    direction,
                                };
                                if let Ok(pkt) = struck_pkt.encode() {
                                    let raw = pkt.encode();
                                    let mut outboxes =
                                        outboxes_for_world_events.lock().unwrap();
                                    for sid in &viewers {
                                        outboxes.entry(*sid).or_default().push(raw.clone());
                                    }
                                }

                                let dmg_pkt = crystal_shared_proto::scene::SDamageIndicator {
                                    damage,
                                    damage_type,
                                    object_id,
                                };
                                if let Ok(pkt) = dmg_pkt.encode() {
                                    let raw = pkt.encode();
                                    let mut outboxes =
                                        outboxes_for_world_events.lock().unwrap();
                                    for sid in &viewers {
                                        outboxes.entry(*sid).or_default().push(raw.clone());
                                    }
                                }
                            }

                            // Always send ObjectHealth for the struck player so
                            // the client can render their head HP bar, and also
                            // broadcast it to any nearby viewers.
                            let health_pkt = crystal_shared_proto::scene::SObjectHealth {
                                object_id,
                                percent: health_percent,
                                expire: 2,
                            };
                            if let Ok(pkt) = health_pkt.encode() {
                                let raw = pkt.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                for sid in &viewers {
                                    outboxes.entry(*sid).or_default().push(raw.clone());
                                }
                                outboxes
                                    .entry(session_id)
                                    .or_default()
                                    .push(raw);
                            }

                            // Always show damage numbers to the struck player,
                            // even if there are no other viewers.
                            let self_dmg_pkt = crystal_shared_proto::scene::SDamageIndicator {
                                damage,
                                damage_type,
                                object_id,
                            };
                            if let Ok(pkt) = self_dmg_pkt.encode() {
                                let raw = pkt.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                outboxes
                                    .entry(session_id)
                                    .or_default()
                                    .push(raw);
                            }

                            // Also update the struck player's own HP/MP bar via SHealthChanged.
                            if let Some((hp, mp)) = hp_mp {
                                let hc_pkt = SHealthChanged { hp, mp };
                                if let Ok(raw) = hc_pkt.encode() {
                                    let encoded = raw.encode();
                                    let mut outboxes =
                                        outboxes_for_world_events.lock().unwrap();
                                    outboxes
                                        .entry(session_id)
                                        .or_default()
                                        .push(encoded);
                                }

                                if hp <= 0 {
                                    if !viewers.is_empty() {
                                        let died_pkt = SObjectDied {
                                            object_id,
                                            location_x: x,
                                            location_y: y,
                                            direction,
                                            death_type: 0,
                                        };
                                        if let Ok(raw) = died_pkt.encode() {
                                            let encoded = raw.encode();
                                            let mut outboxes =
                                                outboxes_for_world_events.lock().unwrap();
                                            for sid in &viewers {
                                                outboxes
                                                    .entry(*sid)
                                                    .or_default()
                                                    .push(encoded.clone());
                                            }
                                        }
                                    }

                                    let self_death = SDeath {
                                        location_x: x,
                                        location_y: y,
                                        direction,
                                    };
                                    if let Ok(raw) = self_death.encode() {
                                        let encoded = raw.encode();
                                        let mut outboxes =
                                            outboxes_for_world_events.lock().unwrap();
                                        outboxes
                                            .entry(session_id)
                                            .or_default()
                                            .push(encoded);
                                    }
                                }
                            }
                        }
                        world::WorldEvent::PlayerHealed {
                            session_id,
                            map_index,
                            x,
                            y,
                            amount: _,
                            new_hp: _,
                        } => {
                            // Send HP/MP update to the healed player.
                            let (hp, mp) = {
                                let w = world_for_tick.lock().unwrap();
                                w.player_current_hp_mp(session_id)
                                    .unwrap_or((0, 0))
                            };

                            let hc_pkt = SHealthChanged { hp, mp };
                            if let Ok(raw) = hc_pkt.encode() {
                                let encoded = raw.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();
                                outboxes
                                    .entry(session_id)
                                    .or_default()
                                    .push(encoded);
                            }

                            // Emit a Healing visual effect for the player and
                            // nearby viewers, approximating the behaviour of
                            // C# SpellEffect.Healing triggered by SafeZone
                            // Healing spell objects.
                            let viewers = {
                                let w = world_for_tick.lock().unwrap();
                                w.sessions_in_range_for_map(
                                    map_index,
                                    x,
                                    y,
                                    LoginConnection::DATA_RANGE,
                                )
                            };

                            // SpellEffect.Healing has enum value 3 in the
                            // original C# client.
                            const HEALING_EFFECT: u8 = 3;
                            let eff_pkt = SObjectEffect {
                                object_id: session_id,
                                effect: HEALING_EFFECT,
                                effect_type: 0,
                                delay_time: 0,
                                time: 0,
                            };

                            if let Ok(raw) = eff_pkt.encode() {
                                let encoded = raw.encode();
                                let mut outboxes =
                                    outboxes_for_world_events.lock().unwrap();

                                // Always show the effect to the healed player.
                                outboxes
                                    .entry(session_id)
                                    .or_default()
                                    .push(encoded.clone());

                                // Also broadcast to other nearby viewers.
                                for sid in viewers {
                                    if sid == session_id {
                                        continue;
                                    }
                                    outboxes
                                        .entry(sid)
                                        .or_default()
                                        .push(encoded.clone());
                                }
                            }
                        }
                        _ => {}
                    }
                }

                thread::sleep(tick);
            }
        });
    }

    // Load experience table from world::configs so we can compute
    // MaxExperience in the same way as the C# Settings.ExperienceList.
    let exp_table: Arc<Vec<i64>> = Arc::new(configs::exp_table().clone());

    let player_summaries: Arc<Mutex<HashMap<world::SessionId, PlayerVisual>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let next_session_id = Arc::new(AtomicU32::new(1));

    let active_connections = Arc::new(AtomicU32::new(0));

    let factory: HandlerFactory = Arc::new({
        let store = Arc::clone(&store);
        let world_db = Arc::clone(&world_db);
        let world_config = world_config.clone();
        let world = Arc::clone(&world);
        let exp_table = Arc::clone(&exp_table);
        let player_summaries = Arc::clone(&player_summaries);
        let next_session_id = Arc::clone(&next_session_id);
        let outboxes = Arc::clone(&outboxes);
        let online_accounts = Arc::clone(&online_accounts);
        let active_connections = Arc::clone(&active_connections);
        let timeout_ms = timeout_ms;
        move || {
            let session_id = next_session_id.fetch_add(1, Ordering::Relaxed);
            Box::new(LoginConnection::new(
                session_id,
                Arc::clone(&store),
                Arc::clone(&world_db),
                world_config.clone(),
                Arc::clone(&world),
                Arc::clone(&exp_table),
                Arc::clone(&player_summaries),
                Arc::clone(&outboxes),
                Arc::clone(&online_accounts),
                Arc::clone(&active_connections),
                timeout_ms,
            )) as Box<dyn ConnectionHandler>
        }
    });

    if let Some(admin_cfg) = admin_cfg.clone() {
        let client = reqwest::Client::new();
        let world_for_metrics = Arc::clone(&world);
        let active_for_metrics = Arc::clone(&active_connections);
        let token = admin_cfg.admin_token.clone();
        let base = format!("http://{}", admin_cfg.admin_listen_addr);
        let metrics_url = format!("{}/internal/metrics", base);

        tokio::spawn(async move {
            loop {
                let snapshot = {
                    let w = world_for_metrics.lock().unwrap();
                    let conn_count = active_for_metrics.load(Ordering::Relaxed);
                    w.snapshot_metrics(conn_count)
                };

                let payload = AdminMetrics {
                    players: snapshot.players,
                    monsters: snapshot.monsters,
                    connections: snapshot.connections,
                    blocked_ips: snapshot.blocked_ips,
                    uptime_seconds: snapshot.uptime_seconds,
                    cycle_delay_ms: snapshot.cycle_delay_ms,
                };

                let res = client
                    .post(&metrics_url)
                    .header("X-Admin-Token", &token)
                    .json(&payload)
                    .send()
                    .await;

                if let Err(e) = res {
                    tracing::debug!("failed to post metrics to admin: {}", e);
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        let client = reqwest::Client::new();
        let world_for_settings = Arc::clone(&world);
        let token = admin_cfg.admin_token.clone();
        let world_settings_url = format!("{}/internal/world-settings", base);

        tokio::spawn(async move {
            loop {
                let res = client
                    .get(&world_settings_url)
                    .header("X-Admin-Token", &token)
                    .send()
                    .await;

                match res {
                    Ok(resp) => {
                        match resp.json::<AdminWorldSettings>().await {
                            Ok(ws) => {
                                let mut w = world_for_settings.lock().unwrap();
                                w.set_spawn_config(
                                    ws.spawn_multiplier,
                                    ws.respawn_base_spawn_rate_minutes,
                                    ws.drop_rate,
                                );
                            }
                            Err(e) => {
                                tracing::debug!("failed to decode world settings from admin: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("failed to fetch world settings from admin: {}", e);
                    }
                }

                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });

        let client = reqwest::Client::new();
        let world_for_players = Arc::clone(&world);
        let world_db_for_players = Arc::clone(&world_db);
        let player_summaries_for_players = Arc::clone(&player_summaries);
        let token = admin_cfg.admin_token.clone();
        let players_url = format!("{}/internal/players", base);

        tokio::spawn(async move {
            loop {
                let core_players = {
                    let w = world_for_players.lock().unwrap();
                    w.snapshot_players()
                };

                let summaries = {
                    let map = player_summaries_for_players.lock().unwrap();
                    map.clone()
                };

                let mut payload: Vec<AdminPlayerInfo> = Vec::new();
                for p in core_players {
                    let visual = summaries.get(&p.session_id);
                    let (name, class_id, gender_id, level) = if let Some(v) = visual {
                        (v.name.clone(), v.class, v.gender, v.level)
                    } else {
                        (format!("Player{}", p.session_id), p.job.as_u8(), 0_u8, p.level)
                    };

                    let class_str = world::Job::from_u8(class_id)
                        .unwrap_or(p.job)
                        .to_string();

                    let gender_str = match gender_id {
                        0 => "Male",
                        1 => "Female",
                        _ => "Unknown",
                    }
                    .to_string();

                    let map_name = world_db_for_players
                        .get_map_info(p.map_index)
                        .map(|m| m.title.clone())
                        .unwrap_or_else(|| format!("Map{}", p.map_index));

                    payload.push(AdminPlayerInfo {
                        id: p.session_id,
                        name,
                        level: level as u32,
                        class: class_str,
                        gender: gender_str,
                        map: map_name,
                    });
                }

                let res = client
                    .post(&players_url)
                    .header("X-Admin-Token", &token)
                    .json(&payload)
                    .send()
                    .await;

                if let Err(e) = res {
                    tracing::debug!("failed to post players to admin: {}", e);
                }

                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });

        let client = reqwest::Client::new();
        let logs_shared = shared_logs.clone();
        let token = admin_cfg.admin_token.clone();
        let logs_url = format!("{}/internal/logs", base);
        let debug_logs_url = format!("{}/internal/debug-logs", base);
        let chat_logs_url = format!("{}/internal/chat-logs", base);

        #[derive(Serialize)]
        struct AdminLogEntry {
            message: String,
        }

        tokio::spawn(async move {
            loop {
                let (logs_vec, debug_vec, chat_vec) = logs_shared.snapshot();

                let logs_payload: Vec<AdminLogEntry> =
                    logs_vec.into_iter().map(|m| AdminLogEntry { message: m }).collect();
                let debug_payload: Vec<AdminLogEntry> = debug_vec
                    .into_iter()
                    .map(|m| AdminLogEntry { message: m })
                    .collect();
                let chat_payload: Vec<AdminLogEntry> = chat_vec
                    .into_iter()
                    .map(|m| AdminLogEntry { message: m })
                    .collect();

                let res_logs = client
                    .post(&logs_url)
                    .header("X-Admin-Token", &token)
                    .json(&logs_payload)
                    .send()
                    .await;
                if let Err(e) = res_logs {
                    tracing::debug!("failed to post logs to admin: {}", e);
                }

                let res_debug = client
                    .post(&debug_logs_url)
                    .header("X-Admin-Token", &token)
                    .json(&debug_payload)
                    .send()
                    .await;
                if let Err(e) = res_debug {
                    tracing::debug!("failed to post debug logs to admin: {}", e);
                }

                let res_chat = client
                    .post(&chat_logs_url)
                    .header("X-Admin-Token", &token)
                    .json(&chat_payload)
                    .send()
                    .await;
                if let Err(e) = res_chat {
                    tracing::debug!("failed to post chat logs to admin: {}", e);
                }

                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });

        let client = reqwest::Client::new();
        let world_for_broadcast = Arc::clone(&world);
        let outboxes_for_broadcast = Arc::clone(&outboxes);
        let token = admin_cfg.admin_token.clone();
        let broadcasts_url = format!("{}/internal/broadcasts", base);

        // ChatType.Shout2 from Shared/Enums.cs
        const CHAT_TYPE_SHOUT2: u8 = 14;

        tokio::spawn(async move {
            loop {
                let res = client
                    .get(&broadcasts_url)
                    .header("X-Admin-Token", &token)
                    .send()
                    .await;

                let Ok(resp) = res else {
                    if let Err(e) = res {
                        tracing::debug!("failed to fetch broadcasts from admin: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                };

                let parsed = resp.json::<Vec<AdminBroadcast>>().await;
                let broadcasts = match parsed {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::debug!("failed to decode broadcasts from admin: {}", e);
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        continue;
                    }
                };

                if !broadcasts.is_empty() {
                    let sessions: Vec<world::SessionId> = {
                        let w = world_for_broadcast.lock().unwrap();
                        w.snapshot_players()
                            .into_iter()
                            .map(|p| p.session_id)
                            .collect()
                    };

                    if !sessions.is_empty() {
                        for b in broadcasts {
                            let msg = b.message.trim();
                            if msg.is_empty() {
                                continue;
                            }

                            let pkt = SChat {
                                message: msg.to_string(),
                                chat_type: CHAT_TYPE_SHOUT2,
                            };

                            if let Ok(raw) = pkt.encode() {
                                let encoded = raw.encode();
                                let mut outboxes = outboxes_for_broadcast.lock().unwrap();
                                for sid in &sessions {
                                    outboxes
                                        .entry(*sid)
                                        .or_default()
                                        .push(encoded.clone());
                                }
                            }
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });
    }

    tracing::info!("Rust Crystal stub server listening on {}", addr);

    run_server(addr, factory, cfg.max_ip, cfg.ip_block_seconds).await
}

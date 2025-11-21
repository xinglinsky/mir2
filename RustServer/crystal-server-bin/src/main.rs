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
use crystal_server_core::world::{self, WorldConfig, WorldDatabase, WorldProvider};
use crystal_server_core::account::AccountStore;
use crystal_server_db::SqliteAccountStore;

use serde::Serialize;
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
    let store: Arc<dyn AccountStore> = Arc::new(
        SqliteAccountStore::open(&cfg.accounts_db_path)
            .expect("failed to open accounts sqlite database"),
    );

    // Load MapInfoList from the C# Server.MirDB database so we can use
    // real map metadata to drive map loading and packets. This keeps
    // compatibility with the existing C# server's database format.
    let mut world_db = WorldDatabase::new();
    match world::map::load_map_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(maps) => {
            println!(
                "[core] Loaded {} MapInfo entries from Server.MirDB",
                maps.len()
            );
            world_db.map_infos = maps;
        }
        Err(e) => {
            println!(
                "[core] Failed to load Server.MirDB (MapInfoList): {} (continuing with empty DB)",
                e
            );
        }
    }

    // Load ItemInfoList so that systems like NPC shops can construct
    // concrete UserItemData payloads for goods, mirroring C# Envir.ItemInfoList.
    match world::map::load_item_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(items) => {
            println!(
                "[core] Loaded {} ItemInfo entries from Server.MirDB",
                items.len()
            );
            world_db.item_infos = items;
        }
        Err(e) => {
            println!(
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
            println!(
                "[core] Loaded {} MonsterInfo entries from Server.MirDB",
                monsters.len()
            );
            world_db.monster_infos = monsters;
        }
        Err(e) => {
            println!(
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
            println!(
                "[core] Loaded {} NPCInfo entries from Server.MirDB",
                npcs.len()
            );
            world_db.npc_infos = npcs;
        }
        Err(e) => {
            println!(
                "[core] Failed to load Server.MirDB (NPCInfoList): {} (continuing without NPC DB)",
                e
            );
        }
    }

    // Load MagicInfoList so that the combat/skill system can access real spell
    // definitions (costs, ranges, power, etc.) from the MirDB.
    match world::map::load_magic_infos_from_mirdb(&cfg.server_mirdb_path) {
        Ok(magics) => {
            println!(
                "[core] Loaded {} MagicInfo entries from Server.MirDB",
                magics.len()
            );
            world_db.magic_infos = magics;
        }
        Err(e) => {
            println!(
                "[core] Failed to load Server.MirDB (MagicInfoList): {} (continuing without Magic DB)",
                e
            );
        }
    }
    let world_db = Arc::new(world_db);

    // Map directory comes from configuration (maps_path), mirroring C# Settings.MapPath.
    let world_config = WorldConfig::new(&cfg.maps_path);

    // Global world instance shared by all connections, mirroring the single-world
    // design of the original C# server. For now this is used synchronously; later
    // we can introduce a dedicated world tick loop and message queues.
    let world = world::World::new((*world_db).clone(), world_config.clone());
    let world = Arc::new(Mutex::new(world));

    // Simple world tick thread driving the World::update loop. For now this
    // only advances time for future respawn/AI logic and does not emit any
    // network events. The tick interval roughly mirrors the C# Envir.Process
    // cadence (50–100ms range).
    {
        let world = Arc::clone(&world);
        thread::spawn(move || {
            let tick = Duration::from_millis(50);
            let start = Instant::now();
            loop {
                let now_ms = start.elapsed().as_millis() as i64;
                {
                    let mut w = world.lock().unwrap();
                    let _events = w.update(now_ms);
                    // TODO: handle world events (monster movement, buffs, etc.).
                }
                thread::sleep(tick);
            }
        });
    }

    // Load experience table (ExpList.ini) so we can compute MaxExperience in
    // the same way as the C# Settings.ExperienceList. We look for the file in
    // ./Configs relative to the current working directory.
    let exp_table: Arc<Vec<i64>> = {
        let mut table = Vec::new();
        let path = "./Configs/ExpList.ini";
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('[') {
                    continue;
                }
                if let Some((_, value)) = line.split_once('=') {
                    if let Ok(v) = value.trim().parse::<i64>() {
                        table.push(v);
                    }
                }
            }
        }
        Arc::new(table)
    };

    let player_summaries: Arc<Mutex<HashMap<world::SessionId, PlayerVisual>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let outboxes: Arc<Mutex<HashMap<world::SessionId, Vec<Vec<u8>>>>> =
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
        let active_connections = Arc::clone(&active_connections);
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
                Arc::clone(&active_connections),
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
    }

    tracing::info!("Rust Crystal stub server listening on {}", addr);

    run_server(addr, factory).await
}

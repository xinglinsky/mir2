use std::{
    fs,
    io,
    net::SocketAddr,
    sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use std::collections::HashMap;

use crate::connection::{LoginConnection, PlayerVisual};

use crystal_server_net::{run_server, ConnectionHandler, HandlerFactory};
use crystal_server_core::world::{self, WorldConfig, WorldDatabase};
use crystal_server_core::account::AccountStore;
use crystal_server_db::SqliteAccountStore;

mod config;
mod connection;


#[tokio::main]
async fn main() -> io::Result<()> {
    let cfg = config::load_server_config("server.toml")
        .expect("failed to load server configuration");

    let filter = tracing_subscriber::EnvFilter::new(
        cfg.log_filter
            .as_deref()
            .unwrap_or("info"),
    );

    tracing_subscriber::fmt()
        .with_env_filter(filter)
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
            loop {
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
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

    let factory: HandlerFactory = Arc::new({
        let store = Arc::clone(&store);
        let world_db = Arc::clone(&world_db);
        let world_config = world_config.clone();
        let world = Arc::clone(&world);
        let exp_table = Arc::clone(&exp_table);
        let player_summaries = Arc::clone(&player_summaries);
        let next_session_id = Arc::clone(&next_session_id);
        let outboxes = Arc::clone(&outboxes);
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
            )) as Box<dyn ConnectionHandler>
        }
    });

    tracing::info!("Rust Crystal stub server listening on {}", addr);

    run_server(addr, factory).await
}

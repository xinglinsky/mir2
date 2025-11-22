use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::RwLock};
use tower_http::services::ServeDir;

mod config;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize, Deserialize, Clone)]
struct Metrics {
    players: u32,
    monsters: u32,
    connections: u32,
    blocked_ips: u32,
    uptime_seconds: u64,
    cycle_delay_ms: u32,
}

#[derive(Serialize, Deserialize, Clone)]
struct WorldSettings {
    spawn_multiplier: u16,
    respawn_base_spawn_rate_minutes: u8,
    drop_rate: f32,
}

#[derive(Serialize, Deserialize, Clone)]
struct LogEntry {
    message: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct PlayerInfo {
    id: u32,
    name: String,
    level: u32,
    class: String,
    gender: String,
    map: String,
}

#[derive(Serialize)]
struct ControlResponse {
    ok: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct BroadcastRequest {
    message: String,
}

#[derive(Serialize)]
struct BroadcastResponse {
    ok: bool,
}

const ADMIN_TOKEN_HEADER: &str = "X-Admin-Token";

fn is_authorized(headers: &HeaderMap, cfg: &config::AdminConfig) -> bool {
    match headers.get(ADMIN_TOKEN_HEADER) {
        Some(value) => value == cfg.admin_token.as_str(),
        None => false,
    }
}

#[derive(Clone)]
struct InnerState {
    metrics: Metrics,
    world_settings: WorldSettings,
    logs: Vec<LogEntry>,
    debug_logs: Vec<LogEntry>,
    chat_logs: Vec<LogEntry>,
    players: Vec<PlayerInfo>,
    pending_broadcasts: Vec<String>,
}

struct AppSharedState {
    inner: RwLock<InnerState>,
    config: config::AdminConfig,
}

type AppState = Arc<AppSharedState>;

impl InnerState {
    fn new() -> Self {
        InnerState {
            metrics: Metrics {
                players: 0,
                monsters: 0,
                connections: 0,
                blocked_ips: 0,
                uptime_seconds: 0,
                cycle_delay_ms: 0,
            },
            world_settings: WorldSettings {
                spawn_multiplier: 1,
                respawn_base_spawn_rate_minutes: 20,
                drop_rate: 1.0,
            },
            logs: vec![LogEntry {
                message: String::from("log stub"),
            }],
            debug_logs: vec![LogEntry {
                message: String::from("debug log stub"),
            }],
            chat_logs: vec![LogEntry {
                message: String::from("chat log stub"),
            }],
            players: vec![PlayerInfo {
                id: 1,
                name: String::from("StubPlayer"),
                level: 1,
                class: String::from("Warrior"),
                gender: String::from("Male"),
                map: String::from("StubMap"),
            }],
            pending_broadcasts: Vec::new(),
        }
    }
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Metrics>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let inner = state.inner.read().await;
    Ok(Json(inner.metrics.clone()))
}

async fn world_settings_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<WorldSettings>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let inner = state.inner.read().await;
    Ok(Json(inner.world_settings.clone()))
}

async fn world_settings_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<WorldSettings>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.inner.write().await;
    inner.world_settings = payload;
    Ok(Json(ControlResponse { ok: true }))
}

async fn logs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let inner = state.inner.read().await;
    Ok(Json(inner.logs.clone()))
}

async fn debug_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let inner = state.inner.read().await;
    Ok(Json(inner.debug_logs.clone()))
}

async fn chat_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let inner = state.inner.read().await;
    Ok(Json(inner.chat_logs.clone()))
}

async fn players(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PlayerInfo>>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let inner = state.inner.read().await;
    Ok(Json(inner.players.clone()))
}

async fn control_start(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    println!("control: start server");
    Ok(Json(ControlResponse { ok: true }))
}

async fn control_stop(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    println!("control: stop server");
    Ok(Json(ControlResponse { ok: true }))
}

async fn control_reboot(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    println!("control: reboot server");
    Ok(Json(ControlResponse { ok: true }))
}

async fn control_clear_blocked_ips(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    println!("control: clear blocked IPs");
    Ok(Json(ControlResponse { ok: true }))
}

async fn reload_npcs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    println!("reload: NPCs");
    Ok(Json(ControlResponse { ok: true }))
}

async fn reload_drops(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    println!("reload: drops");
    Ok(Json(ControlResponse { ok: true }))
}

async fn reload_line_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    println!("reload: line messages");
    Ok(Json(ControlResponse { ok: true }))
}

async fn broadcast(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<BroadcastRequest>,
) -> Result<Json<BroadcastResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let msg = req.message.trim();
    if msg.len() < 5 {
        return Err(StatusCode::BAD_REQUEST);
    }

    {
        let mut inner = state.inner.write().await;
        inner.pending_broadcasts.push(msg.to_string());
    }

    println!("broadcast: {}", msg);
    Ok(Json(BroadcastResponse { ok: true }))
}

async fn set_metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Metrics>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.inner.write().await;
    inner.metrics = payload;
    Ok(Json(ControlResponse { ok: true }))
}

async fn set_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Vec<LogEntry>>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.inner.write().await;
    inner.logs = payload;
    Ok(Json(ControlResponse { ok: true }))
}

async fn set_debug_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Vec<LogEntry>>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.inner.write().await;
    inner.debug_logs = payload;
    Ok(Json(ControlResponse { ok: true }))
}

async fn set_chat_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Vec<LogEntry>>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.inner.write().await;
    inner.chat_logs = payload;
    Ok(Json(ControlResponse { ok: true }))
}

async fn set_players(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Vec<PlayerInfo>>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.inner.write().await;
    inner.players = payload;
    Ok(Json(ControlResponse { ok: true }))
}

async fn take_broadcasts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<BroadcastRequest>>, StatusCode> {
    if !is_authorized(&headers, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let mut inner = state.inner.write().await;
    let mut out = Vec::new();
    for msg in inner.pending_broadcasts.drain(..) {
        out.push(BroadcastRequest {
            message: msg,
        });
    }

    Ok(Json(out))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::load_admin_config("admin.toml")?;
    let addr: SocketAddr = cfg.admin_listen_addr;

    let state: AppState = Arc::new(AppSharedState {
        inner: RwLock::new(InnerState::new()),
        config: cfg,
    });

    let static_dir = format!("{}/static", env!("CARGO_MANIFEST_DIR"));
    let static_service = ServeDir::new(static_dir).append_index_html_on_directories(true);

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/world/settings", get(world_settings_get).post(world_settings_set))
        .route("/logs", get(logs))
        .route("/debug-logs", get(debug_logs))
        .route("/chat-logs", get(chat_logs))
        .route("/players", get(players))
        .route("/control/start", post(control_start))
        .route("/control/stop", post(control_stop))
        .route("/control/reboot", post(control_reboot))
        .route("/control/clear-blocked-ips", post(control_clear_blocked_ips))
        .route("/reload/npcs", post(reload_npcs))
        .route("/reload/drops", post(reload_drops))
        .route("/reload/line-messages", post(reload_line_messages))
        .route("/broadcast", post(broadcast))
        .route("/internal/metrics", post(set_metrics))
        .route("/internal/logs", post(set_logs))
        .route("/internal/debug-logs", post(set_debug_logs))
        .route("/internal/chat-logs", post(set_chat_logs))
        .route("/internal/players", post(set_players))
        .route("/internal/world-settings", get(world_settings_get))
        .route("/internal/broadcasts", get(take_broadcasts))
        .nest_service("/", static_service)
        .with_state(state);
    println!("Crystal admin web console listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

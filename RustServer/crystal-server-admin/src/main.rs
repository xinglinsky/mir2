use std::{net::SocketAddr, sync::Arc};

use axum::{
    routing::{get, post},
    extract::State,
    http::{HeaderMap, StatusCode},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::RwLock};
use tower_http::services::ServeDir;

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

#[derive(Deserialize)]
struct BroadcastRequest {
    message: String,
}

#[derive(Serialize)]
struct BroadcastResponse {
    ok: bool,
}

const ADMIN_TOKEN_HEADER: &str = "X-Admin-Token";
const ADMIN_TOKEN_VALUE: &str = "change-me";

fn is_authorized(headers: &HeaderMap) -> bool {
    match headers.get(ADMIN_TOKEN_HEADER) {
        Some(value) => value == ADMIN_TOKEN_VALUE,
        None => false,
    }
}

#[derive(Clone)]
struct InnerState {
    metrics: Metrics,
    logs: Vec<LogEntry>,
    debug_logs: Vec<LogEntry>,
    chat_logs: Vec<LogEntry>,
    players: Vec<PlayerInfo>,
}

type AppState = Arc<RwLock<InnerState>>;

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
        }
    }
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn metrics(State(state): State<AppState>) -> Json<Metrics> {
    let inner = state.read().await;
    Json(inner.metrics.clone())
}

async fn logs(State(state): State<AppState>) -> Json<Vec<LogEntry>> {
    let inner = state.read().await;
    Json(inner.logs.clone())
}

async fn debug_logs(State(state): State<AppState>) -> Json<Vec<LogEntry>> {
    let inner = state.read().await;
    Json(inner.debug_logs.clone())
}

async fn chat_logs(State(state): State<AppState>) -> Json<Vec<LogEntry>> {
    let inner = state.read().await;
    Json(inner.chat_logs.clone())
}

async fn players(State(state): State<AppState>) -> Json<Vec<PlayerInfo>> {
    let inner = state.read().await;
    Json(inner.players.clone())
}

async fn control_start() -> Json<ControlResponse> {
    println!("control: start server");
    Json(ControlResponse { ok: true })
}

async fn control_stop() -> Json<ControlResponse> {
    println!("control: stop server");
    Json(ControlResponse { ok: true })
}

async fn control_reboot() -> Json<ControlResponse> {
    println!("control: reboot server");
    Json(ControlResponse { ok: true })
}

async fn control_clear_blocked_ips() -> Json<ControlResponse> {
    println!("control: clear blocked IPs");
    Json(ControlResponse { ok: true })
}

async fn reload_npcs() -> Json<ControlResponse> {
    println!("reload: NPCs");
    Json(ControlResponse { ok: true })
}

async fn reload_drops() -> Json<ControlResponse> {
    println!("reload: drops");
    Json(ControlResponse { ok: true })
}

async fn reload_line_messages() -> Json<ControlResponse> {
    println!("reload: line messages");
    Json(ControlResponse { ok: true })
}

async fn broadcast(Json(req): Json<BroadcastRequest>) -> Json<BroadcastResponse> {
    println!("broadcast: {}", req.message);
    Json(BroadcastResponse { ok: true })
}

async fn set_metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Metrics>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.write().await;
    inner.metrics = payload;
    Ok(Json(ControlResponse { ok: true }))
}

async fn set_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Vec<LogEntry>>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.write().await;
    inner.logs = payload;
    Ok(Json(ControlResponse { ok: true }))
}

async fn set_debug_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Vec<LogEntry>>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.write().await;
    inner.debug_logs = payload;
    Ok(Json(ControlResponse { ok: true }))
}

async fn set_chat_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Vec<LogEntry>>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.write().await;
    inner.chat_logs = payload;
    Ok(Json(ControlResponse { ok: true }))
}

async fn set_players(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Vec<PlayerInfo>>,
) -> Result<Json<ControlResponse>, StatusCode> {
    if !is_authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut inner = state.write().await;
    inner.players = payload;
    Ok(Json(ControlResponse { ok: true }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state: AppState = Arc::new(RwLock::new(InnerState::new()));

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
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
        .nest_service("/", ServeDir::new("static"))
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:7001".parse()?;
    println!("Crystal admin web console listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

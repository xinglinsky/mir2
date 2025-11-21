# Mir2 RustServer Admin 控制台实现计划

> 说明：本计划专门针对服务端后台管理与 dashboard 功能，参考 C# `HttpServer` / `HttpService` 与 Rust `crystal-server-admin` 现状。
> 全程遵守 RUST_SERVER_RULES：仅修改 RustServer 目录；行为对齐 C#；尽量避免 panic；单个 `.rs` 文件不超过 800 行并按职责拆分。

---

## 一、目标与范围

- **参考对象**
  - C#：
    - `Server/Utils/HttpServer.cs`
    - `Server/Utils/HttpService.cs`
    - `Server/MirEnvir/Envir.cs` 中的 `HTTP*` 方法（例如 `HTTPNewAccount`, `HTTPLogin` 等）。
  - Rust：
    - `RustServer/crystal-server-admin/src/main.rs` 及其既有路由：
      - `/health`, `/metrics`, `/logs`, `/debug-logs`, `/chat-logs`, `/players`,
      - `/control/start|stop|reboot|clear-blocked-ips`,
      - `/reload/npcs|drops|line-messages`,
      - `/broadcast`, `/internal/*` 等。

- **本计划范围**
  - 只改动 `RustServer` 内代码，重用/增强 `crystal-server-admin`；不修改 C# 工程。
  - **暂不开发**：NPC / 怪物 / 任务等数据库编辑器（图形/网页编辑界面、保存逻辑等）。
  - **优先实现**：核心管理与 dashboard 功能：
    - 实时 Metrics & 在线玩家列表；
    - 日志/聊天记录查看；
    - 广播消息；
    - 清理封禁 IP；
    - 重载配置/脚本；
    - 发起关服/重启（配合外部进程管理）。

---

## 二、整体架构设计

### 2.1 角色划分

- **游戏服进程**（`crystal-server-bin` + `crystal-server-core`）
  - 负责真正的游戏逻辑与状态：连接、世界、地图、玩家、行会等。
  - 通过内部 HTTP / TCP / IPC 向 Admin：
    - 定期上报 metrics / players / logs / chat_logs。
    - 接收控制指令（stop/reboot/broadcast/reload/...）。

- **Admin Web 进程**（`crystal-server-admin`）
  - 负责：
    - 提供 Web UI（静态前端）与 JSON API；
    - 鉴权与安全控制；
    - 缓存来自游戏服的状态（metrics、logs、players 等）。

### 2.2 数据流

- **读类（Dashboard）**
  - 游戏服 → Admin：
    - 通过 `/internal/*` POST 接口推送最新数据（metrics / logs / players / chat_logs 等）。
  - Admin → 浏览器：
    - 各类 GET 接口（`/metrics`, `/players`, `/logs` 等）从内存 `InnerState` 中读取并返回 JSON。

- **控类（Control）**
  - 浏览器 → Admin：
    - 调用 `/control/*`, `/reload/*`, `/broadcast` 等接口。
  - Admin → 游戏服：
    - 在后端调用游戏服暴露的控制接口（建议为 HTTP 内部端口或 IPC），触发相应行为。

### 2.3 安全基础

- 延续现有 `X-Admin-Token` 鉴权机制，但：
  - Token 值必须从配置文件加载，禁用硬编码 `"change-me"`。
  - 可选增加 IP 白名单机制，对标 C# `HTTPTrustedIPAddress`：仅允许可信 IP 访问管理接口。

---

## 三、阶段一：Admin 基础 & 安全加固（高优先级）

**目标**：让 `crystal-server-admin` 具备可配置的监听地址与 token，所有写操作强制鉴权，并保证结构可维护。

### 3.1 配置整合

- 在 RustServer 配置中引入 Admin 相关配置（可放在 `server.toml` 或专门 `admin.toml`）：
  - `admin_listen_addr`: 字符串，如 `"0.0.0.0:7001"`；
  - `admin_token`: 字符串，用于 `X-Admin-Token` 校验；
  - `admin_trusted_ip`: 可选，单 IP 或 IP 列表；暂时可先实现单一 IP 或前缀匹配。

- 在 `crystal-server-admin/src/main.rs` 中：
  - 替换硬编码的 `ADMIN_TOKEN_VALUE`，改为从配置读取；
  - 监听地址由配置 `admin_listen_addr` 决定，而非硬编码 `0.0.0.0:7001`。

### 3.2 统一鉴权

- 抽象一个鉴权辅助函数或 Axum 中间件：
  - 检查 Header 中的 `X-Admin-Token` 是否匹配配置；
  - 可获取发起请求的 IP 并与 `admin_trusted_ip` 比对，不符合时直接返回 401/403。

- 适用范围：
  - 所有具有**写入/控制**性质的接口：
    - `/internal/*` 全部；
    - `/control/*` 全部；
    - `/reload/*` 全部；
    - `/broadcast`；
  - 只读接口（`/health`, `/metrics`, `/logs`, `/players` 等）可按需求选择是否也要求 token（推荐至少 metrics/players/logs 需要 token）。

### 3.3 结构整理 & 单文件行数控制

- 如 `main.rs` 体量过大（接近 800 行），按职责拆分：
  - `mod config`：读取 admin 配置；
  - `mod auth`：鉴权与 IP 校验；
  - `mod models`：`Metrics`, `LogEntry`, `PlayerInfo`, `ControlResponse` 等；
  - `mod state`：`InnerState` 与共享 `AppState`；
  - `mod routes`：GET/POST 路由注册与 handler。

> 阶段一完成后：Admin 进程可安全对外暴露，攻击面和误操作风险降低。

---

## 四、阶段二：核心 Dashboard（Metrics & 在线玩家）

**目标**：Admin 能展示真实的服务器运行指标与在线玩家列表，替代当前的 stub 数据。

### 4.1 Metrics 数据建模（游戏服侧）

- 在 `crystal-server-core::world` / `connection` 等模块中：
  - 提供一个 snapshot API，例如：

    ```rust
    pub struct CoreMetrics { /* 字段与 admin::Metrics 对齐或可转换 */ }

    impl World {
        pub fn snapshot_metrics(&self) -> CoreMetrics { /* 汇总玩家数、怪物数、连接数、封禁 IP 数、uptime、cycle_delay 等 */ }
    }
    ```

- 字段对齐到 Admin 的 `Metrics` 结构（players, monsters, connections, blocked_ips, uptime_seconds, cycle_delay_ms），可按需扩展。

### 4.2 游戏服→Admin 的 Metrics 上报

- 在 `crystal-server-bin` 中增加后台任务：
  - 每隔 N 秒（例如 1~5 秒）：
    - 调用 `world.snapshot_metrics()`；
    - 通过 HTTP POST 请求 Admin `/internal/metrics`，发送 JSON。

- 错误处理：
  - 遵循“尽量不 panic”的规则：
    - 如果 Admin 暂时不可达，仅记录日志，等待下次重试；
    - 不因上报失败中断游戏服主循环。

### 4.3 在线玩家列表上报

- 在 `crystal-server-core::player` 或 `world` 中：
  - 提供 `snapshot_players()`：

    ```rust
    pub struct CorePlayerInfo { /* 对齐 admin::PlayerInfo 的字段 */ }

    impl World {
        pub fn snapshot_players(&self) -> Vec<CorePlayerInfo> { /* 枚举在线玩家 */ }
    }
    ```

- 在 `crystal-server-bin` 中：
  - 与 metrics 同步或稍低频率（例如 5~10 秒）：
    - 通过 POST `/internal/players` 将玩家列表推送给 Admin。

> 阶段二完成后：Admin `/metrics` `/players` 将展示真实数据，形成基础 dashboard。

---

## 五、阶段三：日志中心（logs / debug-logs / chat-logs）

**目标**：通过 Admin 面板查看最近的系统日志、调试日志和聊天记录，辅助运维与排查。

### 5.1 日志分类与采集

- 在 Rust 游戏服中（`crystal-server-bin` + `core`）：
  - 统一使用 logging 库（如 `tracing` / `log`）并区分 target 或 level：
    - `target="game_log"`
    - `target="debug_log"`
    - `target="chat_log"`

- 选择一种采集方式：
  - 内存环形缓存：
    - 在核心模块中维护最近 N 条日志（如 1000 条），供 snapshot 使用；
  - 或文件 tail + 聚合：
    - 由后台任务从文件尾部读取最近 N 行，并转换为 `LogEntry` 列表。

### 5.2 推送到 Admin

- 在 `crystal-server-bin` 中增加上报任务：
  - 定时或按事件阈值，将 `Vec<LogEntry>` 推送至：
    - `/internal/logs`
    - `/internal/debug-logs`
    - `/internal/chat-logs`

- Admin 端在 `InnerState` 中：
  - 用 `Vec<LogEntry>` 存储各类日志；
  - 可能限制最大长度（超过后丢弃最老的记录）。

### 5.3 前端展示（可逐步增强）

- 现阶段：
  - 确保 GET `/logs` `/debug-logs` `/chat-logs` 返回按时间排序的 JSON 数组。
- 后续可在 `static` 前端：
  - 增加简单的日志页面：列表 + 按关键字过滤 / 按类型切换。

> 阶段三完成后，运维可以使用 Web 看日志，无需登陆服务器读文件。

---

## 六、阶段四：控制与运维操作（Control & Reload & Broadcast）

**目标**：实现实际可用的服务器控制接口，替代当前的 `println!` stub，实现对标 C# HttpServer 的管理能力。

> **注意**：不在本计划中开发“编辑器”，仅提供控制/重载入口。

### 6.1 Broadcast 广播（对齐 C#）

- C# `/broadcast` 行为概览：
  - 从 `request.QueryString["msg"]` 获取消息；
  - 长度 < 5 时返回 `"short"`；
  - 否则调用 `Envir.Main.Broadcast(new S.Chat { Message = msg.Trim(), Type = ChatType.Shout2 })`。

- Rust 实现建议：
  - 保持 Admin `/broadcast` 接口的 JSON 请求体：
    - `{ "message": "..." }`。
  - 在游戏服中实现一个控制入口：
    - 如果共用 HTTP 内部端口，可新增 `/internal/control/broadcast`；
    - Admin 收到 broadcast 请求后，调用该内部接口：
      - 校验长度与内容（<5 返回错误）；
      - 在世界中广播等价的聊天消息（`Shout2` 或对应的 Rust 枚举 variant）。

### 6.2 服务器生命周期控制（start / stop / reboot）

- 推荐进程模型：
  - Admin 与 游戏服由外部进程管理器（如 systemd、Windows 服务、supervisor 或手工脚本）拉起；
  - Admin 不直接 fork/exec 游戏服，而是通过控制接口改变游戏服状态。

- Rust 游戏服内部：
  - `/control/stop`：
    - 触发有序关服流程（对应 C# `StopEnvir`）：
      - 广播关服公告；
      - 停止接受新连接；
      - 保存 DB；
      - 优雅关闭主循环并退出进程（返回 0 或特定退出码）。
  - `/control/reboot`：
    - 逻辑等价 stop + 外部重启：
      - 游戏服内部只负责干净退出；
      - 由外部管理器监测退出并重新拉起进程。
  - `/control/start`：
    - 在“Admin/游戏服分进程”的模型下，一般交给外部实现；
    - 可以在 Admin 中仅作为一个 no-op 或向外部系统发 webhook，当前计划内可保持为 stub 或简单日志。

### 6.3 清理 IP 封禁

- 参考 C#：`Envir.IPBlocks` 与 IP 封禁逻辑。
- 在 Rust `world`/`connection` 模块中：
  - 定义 `fn clear_blocked_ips()`，清理所有当前 IP 封禁状态或按规则部分清理。
- Admin `/control/clear-blocked-ips`：
  - 经鉴权后调用游戏服内部接口，执行上述清理；
  - 返回执行结果，并记录操作日志（谁在何时清除了 IP 封禁）。

### 6.4 配置/脚本重载（不涉及编辑器）

- 对齐 C# 中的 reload 功能（例如重载 NPC 脚本、掉落表、公告/行消息等）：
  - 在 Rust 世界模块中：
    - 提供 `reload_npcs()`, `reload_drops()`, `reload_line_messages()` 等方法：
      - 从现有数据源（DB/配置文件）重新读取配置并应用；
      - 保证在不中断服务的前提下切换新配置，逻辑对齐 C# 实现。

- Admin `/reload/npcs`, `/reload/drops`, `/reload/line-messages`：
  - 经鉴权后调用对应内部接口；
  - 返回操作结果（包括是否成功/失败原因）。

> 阶段四完成后，Admin 可执行大部分日常运维操作：广播、清 IP、重载配置、发起关服/重启等。

---

## 七、明确不在本轮范围内的功能

- **NPC / 怪物 / 地图 / 任务等数据库编辑 UI**：
  - 包括：
    - 通过 Web 创建/编辑 NPC、怪物、地图点、掉落规则；
    - 可视化任务编辑器、脚本编辑器；
    - 任意 DB 表的在线编辑。
  - 这些功能需要：
    - 更复杂的前端交互；
    - 更严格的权限与审计机制；
    - 完整的版本管理/回滚方案。
  - 暂时不在本计划中，未来可作为“Admin 编辑器计划”独立文档。

---

## 八、执行与协作建议

- 所有实现遵循：
  - 仅修改 `RustServer` 目录内代码；
  - 严格参考 C# 行为实现 Rust 等价逻辑；
  - 网络/配置/日志/控制接口均不使用 `panic!`、`unwrap()`、`expect()` 处理外部输入，统一使用 `Result` + 日志；
  - 单个 `.rs` 文件不超过 800 行，按职责拆分模块。

- 建议工作分拆：
  - **A 组**：负责 Stage 1+2（配置/安全 + Metrics & Players）；
  - **B 组**：负责 Stage 3（日志中心）；
  - **C 组**：负责 Stage 4（控制与重载接口）。

- 建议每个阶段以独立分支或 PR 形式提交，包含：
  - 对应路由与 handler 改动；
  - 与游戏服交互的适配；
  - 基本单元测试/集成测试，至少覆盖：
    - 鉴权逻辑；
    - Metrics/Players JSON 结构；
    - 控制接口对错误情况的处理（Admin/游戏服不可达时的表现）。

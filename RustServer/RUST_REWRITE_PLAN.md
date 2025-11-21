# Mir2 RustServer 重写总体计划

> 本计划遵循 `RUST_SERVER_RULES`（行为与 C# 保持一致、仅修改 RustServer 目录、错误处理避免 panic、单文件不超过 800 行并按职责拆分模块等）。

## 一、总体依赖关系

- **子计划1：协议 & 连接层重写**  
  对齐 C# `MirConnection` 的连接生命周期和包分发行为，稳定 `RawPacket` / `ClientPacketIds` / `ServerPackets` 协议与连接状态机。

- **子计划2：世界时间 & 地图系统重写**  
  对齐 C# `Envir` 主循环、时间系统和 `Map` 地图加载/判定逻辑。

- **子计划3：玩家 / 物品 / 基础战斗逻辑**  
  对齐 `PlayerObject`、物品系统与基础战斗相关包，构建最小可玩闭环。

- **子计划4：高级系统 & 运维/Admin 整合**  
  迁移行会、攻城、任务等高级玩法，并与 Rust Admin Web 控制台打通。

建议执行顺序：1 → 2 → 3 → 4，其中 1 与 2 可大部分并行，4 明显依赖前 3。

---

## 二、子计划1：协议 & 连接层重写

### 目标

- 迁移 C# `Server/MirNetwork/MirConnection.cs` 的核心行为到 Rust：
  - 连接生命周期：建立连接、发送初始包、心跳 & 超时、断线清理。
  - 包分发：基于 `ClientPacketIds` 的 `switch` / `match` 分发逻辑。
- 对齐网络协议：`RawPacket`、Client/Server 包 ID 与字段布局完全等价 C#。
- 在 Rust 侧建立稳定的“连接 → 世界逻辑”桥接层，供后续子计划共用。

### 主要涉及代码

- **C# 参考**
  - `Server/MirNetwork/MirConnection.cs`
  - `Server/Settings.cs`（`Network` 部分配置）
  - ClientPackets/ServerPackets 定义（包 ID 与结构）

- **Rust 目标位置（仅限 RustServer 目录）**
  - `crystal-shared-proto`：
    - 协议包 ID 与结构定义（使用 `#[repr(i16)]` 等保证数值一致）。
  - `crystal-server-net`：
    - 已提供 `ConnectionHandler` trait 与 `run_server`，必要时小范围增强（不改行为）。
  - `crystal-server-bin/src/connection.rs`：
    - 实现等价 `MirConnection` 的状态机和 `handle_packet` 分发逻辑。
  - `crystal-server-core` 中视需要新增 `session`/`connection` 接口层（只作为世界接口，不放具体世界逻辑）。

### 执行步骤

1. **协议 ID 与结构对齐**
   - 从 C# ClientPackets / ServerPackets 中提取全部包 ID 与字段布局。
   - 在 `crystal-shared-proto` 中：
     - 使用 `#[repr(i16)]` 的 `enum` 或常量，确保 ID 数值与 C# 完全一致。
     - 为关键登录相关包（ClientVersion, NewAccount, Login, NewCharacter, StartGame 等）实现编解码，配套单元测试。

2. **连接状态机迁移**
   - 在 `connection.rs` 中定义等价 `GameStage`：`Login / Select / Game / Observer / Disconnected`。
   - 迁移构造逻辑：
     - 记录 IP、更新 IP 封锁（对应 C# `Envir.UpdateIPBlock` 行为）。
     - 设置超时（`TimeOutTime` 等）并发送初始 `S.Connected` 包。
   - 对齐 `Disconnecting` 与超时处理逻辑。

3. **`handle_packet` 分发骨架**
   - 优先实现登录/账号相关包分支：
     - `ClientVersion`, `NewAccount`, `ChangePassword`, `Login`, `NewCharacter`, `DeleteCharacter`, `StartGame`, `LogOut` 等。
   - 对于尚未迁移的战斗/物品类包：
     - 在 `match` 中保留分支与 TODO 占位，先以“忽略请求或简单错误响应”的行为保持服务器稳定。

4. **与世界模块的接口设计**
   - 在 `crystal-server-core` 中定义抽象接口（如 `WorldService` trait）：
     - `login_account`, `create_character`, `enter_game` 等方法。
   - 此计划只定义 trait 和调用，具体世界逻辑实现推迟到子计划2/3。

### 与其他子计划的边界

- 本子计划主要修改：
  - `crystal-shared-proto`（协议定义）
  - `crystal-server-net`（传输层小范围增强）
  - `crystal-server-bin/src/connection.rs` 及紧邻模块
- 不实现世界/地图/玩家/战斗逻辑，只通过 trait 与这些模块交互，避免与子计划2/3 冲突。

---

## 三、子计划2：世界时间 & 地图系统重写

### 目标

- 在 Rust 中实现等价 C# `Envir` 的世界时间与主循环：
  - 使用单调毫秒时间，对齐 `Envir.Time = Stopwatch.ElapsedMilliseconds` 语义。
  - 实现 tick 驱动的各种定时器（刷怪、清理、buff 等）的框架。
- 完整迁移 `Map` 地图系统：
  - 多种 map 文件格式识别与解析（参考 C# `Map.FindType` 等）。
  - 可行走/阻挡判定在相同地图与坐标下与 C# 保持一致。

### 主要涉及代码

- **C# 参考**
  - `Server/MirEnvir/Envir.cs`：
    - 时间管理、主循环、地图/Respawn 管理、启动/停止网络（仅作行为参考）。
  - `Server/MirEnvir/Map.cs`：
    - 地图加载、类型识别、Cell/Doors/Mine 等结构与判定。
  - `Server.MirDatabase` 中与 Map/Respawn 相关的数据结构与读写。

- **Rust 目标位置（仅限 RustServer 目录）**
  - `crystal-server-core::world`：
    - 世界状态、时间字段、tick 主循环入口。
  - `crystal-server-core::world::map`：
    - 地图加载、格式识别、可行走/阻挡判定逻辑。
  - 现有 `load_map_infos_from_mirdb` 等函数在此基础上扩展。

### 执行步骤

1. **世界时间与主循环**
   - 在 core 中定义 `World`/`Envir` 等价结构：
     - 字段包含：当前毫秒时间、地图列表、定时器（刷怪/清理等）。
     - 方法 `tick(now_ms: u64)`：复制 C# 中各类 `if (Time > NextX)` 判定与更新顺序。
   - 在 `crystal-server-bin/main.rs` 中：
     - 使用 Tokio 定时任务或 loop 驱动 `world.tick`，保持行为与 C# 主循环相似。

2. **地图加载与格式识别**
   - 迁移 `Map.FindType` 以及不同 map 格式对应的解析代码。
   - 加载结果需保证：
     - 地图宽高、障碍/可行走信息、门/矿等数据与 C# 完全一致。
   - 提供接口：
     - `is_walkable(map_id, x, y)`
     - `can_move(map_id, from, to)`
     - 用作玩家/怪物行为的基础判定。

3. **Respawn 与时间驱动骨架**
   - 定义 Respawn/刷怪配置结构，对应 C# 中相关字段。
   - 在 `world.tick` 中挂上刷怪、地图周期事件的调用点：
     - 暂时可以使用占位怪物/对象，详细怪物逻辑在子计划3中实现。

### 与其他子计划的边界

- 主要修改 `crystal-server-core::world` / `world::map` 以及 `crystal-server-bin/main.rs` 驱动部分。
- 不直接修改连接层 `connection.rs`（只被调用）。
- 不实现 Player/Monster 详细行为，仅提供地图与时间服务，避免与子计划3 冲突。

---

## 四、子计划3：玩家 / 物品 / 基础战斗逻辑

### 目标

- 迁移 C# `PlayerObject` 的核心逻辑：
  - 属性、状态、位置、地图交互等。
- 迁移物品系统与基础交互：
  - 背包/仓库、装备、拾取/丢弃等。
- 实现基础战斗闭环：
  - 移动/转向/近战攻击/简单远程攻击，伤害与经验计算对齐 C#。
- 回填子计划1 中各类 ClientPacketIds 分支，使玩家可以基本“进游戏 → 移动 → 打怪 → 拾取 → 升级”。

### 主要涉及代码

- **C# 参考**
  - `Server/MirObjects/PlayerObject.cs` 及相关对象基类。
  - 物品与背包逻辑：`ItemInfo`, `UserItem`, 仓库/装备等。
  - `MirConnection.ProcessPacket` 中与移动/物品/战斗相关的包处理。

- **Rust 目标位置（仅限 RustServer 目录）**
  - `crystal-server-core::player`：
    - 玩家属性、状态机、行为方法（移动、攻击、捡物等）。
  - `crystal-server-core::item`：
    - 物品数据结构、背包/仓库/装备栏逻辑。
  - `crystal-server-core::combat`：
    - 基础伤害/命中/经验算法，与 C# 对齐。
  - 通过子计划1中定义的接口，连接到 `connection.rs` 的包分发中。

### 执行步骤

1. **Player 结构与生命周期**
   - 在 core 中定义等价 `PlayerObject` 的结构体：
     - 包含等价的属性字段（等级、职业、HP/MP、位置、buff 等）。
   - 与 world/map 对接：
     - 提供在地图上添加/移除玩家、广播位置/动作变化的接口。

2. **物品系统迁移**
   - 在已加载的 `ItemInfoList` 基础上，实现 `UserItem` 与：
     - 背包/仓库/装备栏容量和索引策略。
     - 基础操作：`MoveItem`, `StoreItem`, `SplitItem`, `EquipItem`, `RemoveItem`, `DropItem`, `PickUp` 等。
   - 确保所有数值与约束（容量、重量、绑定状态等）与 C# 一致。

3. **基础战斗逻辑**
   - 迁移 C# 中最核心的近战/简单远程战斗：
     - `Turn`, `Walk`, `Run`, `Attack`, `RangeAttack`。
   - 对齐：
     - 目标选择（前方格子/射线）、障碍判定（依赖 world::map）、
     - 命中/伤害计算、经验分配、死亡与掉落触发点（掉落实现在可只搭建框架）。

4. **回填网络处理**
   - 在 `connection.rs` 内，将上述逻辑接线到各自的 ClientPacket 分支：
     - 保持调用顺序与 C# 中 `ProcessPacket` 一致。
   - 为关键操作增加日志，便于对照 C# 行为进行回归测试。

### 与其他子计划的边界

- 核心修改集中在 `crystal-server-core::player` / `item` / `combat`，
  - 仅通过已定义接口操作 `world`、`map` 模块。
- 在 `connection.rs` 中只增加对新模块的调用，不改动协议基础结构与通用状态机（减少与子计划1 冲突）。

---

## 五、子计划4：高级系统 & 运维/Admin 整合

### 目标

- 将 C# 中的高级系统迁移到 Rust：行会、攻城战、任务、排行榜等。
- 让 Rust 的 `crystal-server-admin` Web 控制台具备与 C# HttpServer 相当的运维功能：
  - 查看状态/在线玩家/日志。
  - 控制服务器启动/停止/重载脚本等。

### 主要涉及代码

- **C# 参考**
  - 行会/攻城：各类 `*Guild*`, `*Conquest*` 相关文件。
  - 任务/龙/魔法信息等：`QuestInfoList`, `DragonInfo`, `MagicInfoList` 等。 
  - HTTP/运维：C# `HttpServer`。

- **Rust 目标位置（仅限 RustServer 目录）**
  - `crystal-server-core` 新增模块：`guild`, `conquest`, `quest`, `ranking` 等。
  - `crystal-server-admin`：
    - 已存在路由（health/metrics/logs/players/control/reload/internal 等），
    - 需补充调用 core/world 的具体实现。

### 执行步骤

1. **高级系统结构迁移**
   - 为行会、攻城、任务、排行榜等建立等价的 Rust 数据结构与状态机。
   - 迁移相关 ClientPacketIds 与服务端逻辑到 core 的这些模块中。

2. **Admin Web 控制台整合**
   - 对齐 C# HttpServer 提供的管理能力：
     - 获取服务器状态、在线玩家数量、关键统计与日志。
     - 控制服务器：重启、清理 IP 封禁、重载 NPC/掉落/行消息等。
   - 在 `crystal-server-admin` 中通过 HTTP API 调用 core/world 接口，实现上述功能。

### 与其他子计划的边界

- 本子计划主要在新建的高级模块与 `crystal-server-admin` 中工作。
- 对前 3 个子计划的模块仅增加调用入口，不修改其内部实现，保证互不冲突。

---

## 六、执行与协作建议
 
 - 所有实现必须遵守已定义的 **RustServer 重写规则**：
   - 仅修改 `RustServer` 目录；
   - 严格按 C# 原有逻辑重写，可优化实现但不改变行为；
   - 包格式、时间/随机、并发语义保持一致；
   - 尽量避免 panic/unwrap/expect，使用 `Result` + 日志 + 安全降级；
   - 每个 `.rs` 文件不超过 800 行，按职责拆分模块。
 
 - 建议为每个子计划维护独立分支或 PR，按模块划分评审范围，便于多人/多 AI 并行推进。

---

## 七、大文件拆分计划（RustServer）
 
 > 目标：控制单个 `.rs` 文件行数（建议 < 800 行），按“职责单一、修改局部化”的原则拆分已有超大文件，降低后续重写和维护成本。
 
 ### 7.1 覆盖范围与当前超限文件
 
 当前在 `RustServer` 目录（排除 `target/`）中，已识别出行数 > 600 行的核心文件：
 
 - `crystal-shared-proto/src/user.rs`（约 3660 行）
 - `crystal-server-core/src/world/map/mirdb.rs`（约 1193 行）
 - `crystal-shared-proto/src/login.rs`（约 1092 行）
 - `crystal-server-bin/src/connection/handler.rs`（约 1066 行）
 - `crystal-shared-proto/src/npc.rs`（约 1056 行）
 - `crystal-shared-proto/src/magic.rs`（约 856 行）
 - `crystal-shared-proto/src/item.rs`（约 627 行）
 
 其中：
 
 - `handler.rs` 与 `user.rs` **承担多种职责、未来改动频繁**，为首要拆分对象。
 - 其余协议/解析类文件复杂度相对集中，可视重写推进情况择机拆分。
 
 ### 7.2 拆分原则
 
 - **行为不变**：拆分仅重构模块边界，不改变对外 API 和协议二进制布局。
 - **按职责聚合**：一个文件只负责一类清晰的概念（如“登录阶段处理”“玩家基础信息包”“MirDB 解析”）。
 - **入口收敛**：对上层调用暴露的路径尽量保持不变，通过 `mod.rs` + `pub use` 重新导出。
 - **阈值控制**：拆分后目标是：
   - 链接层 / 世界逻辑类文件 ≈ 300–600 行；
   - 协议定义类文件 ≈ 300–900 行（可以稍宽松，但不再出现 3000+ 行巨型文件）。
 
 ### 7.3 文件级拆分方案（已完成）
 
 #### 7.3.1 `crystal-server-bin/src/connection/handler.rs`
 
 **关联子计划**：子计划 1（协议 & 连接层重写）、子计划 3（玩家 / 基础战斗）。
 
 **现状简述**：
 
 - `impl ConnectionHandler for LoginConnection` 中的 `handle_packet` 使用一个巨大的 `match ClientPacketId`：
   - 同时处理：账号注册/登录、角色列表、新建/删除角色、StartGame；
   - 同时处理：游戏内移动/转向/聊天/战斗等；
   - 同时管理：连接阶段 `Stage`、角色/会话状态、与 world 的交互和可见性更新。
 
 **拆分目标**：
 
 - 让 `handler.rs` 只承担“入口 + 基础路由”职责，细节逻辑按阶段拆分到子模块。
 - 降低单文件行数，同时为后续扩展新的包类型预留空间。
 
 **拟新增子模块（同目录内新建文件）**：
 
 - `login_stage.rs`：
   - 处理 `ClientVersion` / `NewAccount` / `ChangePassword` / `Login` 等账号与版本验证逻辑。
 - `select_stage.rs`：
   - 处理 `NewCharacter` / `DeleteCharacter` / `StartGame` 前角色选择阶段的逻辑。
 - `ingame_stage.rs`：
   - 处理 `Turn` / `Walk` / `Run` / `Chat` / `Attack` / `CallNPC` 等 InGame 阶段逻辑；
   - 封装地图切换、可见性更新、Save 角色位置/魔法等。
 
 **执行步骤**：
 
 1. **建立阶段子模块骨架**（不改行为，仅移动代码）：
    - 在 `crystal-server-bin/src/connection/` 目录下新建：
      - `login_stage.rs`
      - `select_stage.rs`
      - `ingame_stage.rs`
    - 在现有连接模块入口（`connection` 目录下的 `mod.rs` 或等价文件）中：
      - `mod login_stage;`
      - `mod select_stage;`
      - `mod ingame_stage;`
 
 2. **按阶段提取方法**：
    - 在每个子模块中为 `LoginConnection` 实现独立方法，例如：
      - `fn handle_new_account(&mut self, msg: CNewAccount, out: &mut Vec<Vec<u8>>)`
      - `fn handle_login(&mut self, msg: CLogin, out: &mut Vec<Vec<u8>>)`
      - `fn handle_start_game(&mut self, msg: CStartGame, out: &mut Vec<Vec<u8>>)`
      - `fn handle_turn(&mut self, msg: CTurn, out: &mut Vec<Vec<u8>>)` 等。
    - 将 `handler.rs` 中对应 `match` 分支的大块逻辑整体剪切到这些方法中，
      - 保持对 `self.store` / `self.world` / `self.stage` 等状态字段的使用方式完全不变。
 
 3. **精简 `handle_packet`**：
    - 在 `handler.rs` 中保留完整的 `match ClientPacketId`，但每个分支仅负责：
      - 解码 `C*` 消息（`CLogin::decode` 等）；
      - 调用对应的 `self.handle_xxx(...)`；
    - 暂不改变任何分支的语义与返回的 `S*` 包序列。
 
 4. **验证与后续优化**：
    - 运行现有登录/进游戏流程，确保行为与拆分前一致（可以通过日志和客户端表现对比）。
    - 后续可以在各子模块内进一步细分：
      - 如把移动/战斗相关逻辑迁移到 `crystal-server-core::player` / `combat`，
      - 但这些属于子计划 3 的范围，在本轮拆分中只做模块边界调整。
 
 #### 7.3.2 `crystal-shared-proto/src/user.rs`
 
 **关联子计划**：子计划 1（协议 & 连接层）、子计划 3（玩家相关协议）。
 
 **现状简述**：
 
 - 包含大量 `S*`/`C*` 结构体及其 `encode/decode` 实现：
   - 玩家完整信息 `SUserInformation`；
   - 场景对象相关：`SObjectPlayer` / `SObjectHero` / `SObjectRemove` / `SObjectTurn/Walk/Run` / `SPushed` 等；
   - 以及更多玩家状态、经验、等级等相关包（依赖 `ServerPacketId` 中的 ID）。
 - 文件内既有“登录阶段初始信息包”，也有“场景内对象广播”，职责跨度过大。
 
 **拆分目标**：
 
 - 将 `user.rs` 拆为一个目录模块 `user/`，按概念域划分子模块：
   - 玩家基础信息/位置；
   - 玩家在场景中的对象表现；
   - 其他与玩家紧密相关但非物品/魔法的协议。
 - 对外仍保留 `use crystal_shared_proto::user::SUserInformation` 等用法，不破坏现有调用。
 
 **拟重构结构**：
 
 ```text
 crystal-shared-proto/src/user/
   mod.rs          // 统一 re-export
   information.rs  // SUserInformation 及相关完整信息包
   location.rs     // SUserLocation 等简单位置更新包
   scene_object.rs // SObjectPlayer/SObjectHero/SObjectRemove/SObjectTurn/Walk/Run/SPushed 等
   status.rs       // 经验/等级/HPMP/中毒/隐藏等玩家状态相关包（如未来存在）
 ```
 
 **执行步骤**：
 
 1. **建立 `user/` 目录模块**：
    - 新建目录：`crystal-shared-proto/src/user/`；
    - 新建 `mod.rs`，内容仅做 `pub use` 与子模块声明：
      - `mod information; mod location; mod scene_object; mod status;`；
      - `pub use information::SUserInformation;` 等，将现有公开类型重新导出。
 
 2. **按结构迁移类型**：
    - 将 `SUserInformation` 及其 `impl` 移动到 `information.rs`；
    - 将 `SUserLocation` 移动到 `location.rs`；
    - 将 `SObjectPlayer` / `SObjectHero` / `SObjectRemove` / `SObjectTurn/Walk/Run` / `SPushed` 等移动到 `scene_object.rs`；
    - 将与玩家状态变化直接相关的包（如存在的 `SHealthChanged` / `SLevelChanged` 等）放入 `status.rs`。
 
 3. **删除旧的 `user.rs`，保留 `mod user;` 入口**：
    - 在 `crystal-shared-proto/src/lib.rs` 或等价入口文件中保持：`pub mod user;`；
    - 确保所有引用路径仍然通过 `crate::user::...` 工作。
 
 4. **回归测试**：
    - 运行 `crystal-shared-proto` 中现有单元测试（如有），确保所有 encode/decode 行为保持一致；
    - 编译并运行登录/进地图流程，确认 `SUserInformation` 包二进制不变。
 
 #### 7.3.3 `crystal-server-core/src/world/map/mirdb.rs`
 
 **关联子计划**：子计划 2（世界时间 & 地图系统）、子计划 3（玩家 / 物品）。
 
 **现状简述**：
 
 - 集中负责从 C# `Server.MirDB` 文件中加载/跳过多个表：
   - `MapInfoList`、`ItemInfoList`、`MonsterInfoList`、`NPCInfoList`、`QuestInfoList`、`MagicInfoList` 等；
   - 包含多个 `load_*_from_mirdb` 顶层函数 + 大量 `read_*`/`skip_*` 的解析细节。
 - 绝大部分逻辑是顺序读写与错误检查，属于“文件格式解析”单一领域。
 
 **拆分目标**：
 
 - 保持对外统一入口（`load_*_from_mirdb`），
 - 将具体解析实现按实体类型拆分到子模块中，便于局部修改与对照 C# 代码。
 
 **建议结构**：
 
 ```text
 crystal-server-core/src/world/map/mirdb/
   mod.rs        // 暴露 load_*_from_mirdb，内部调用各子模块
   header.rs     // MirDB 头部和通用索引读取逻辑
   map.rs        // read_map_info / read_safe_zone / read_respawn / read_movement 等
   item.rs       // ItemInfo 的读取或跳过逻辑
   monster.rs    // MonsterInfo 的读取或跳过逻辑
   npc.rs        // NpcInfo 的读取逻辑
   quest.rs      // QuestInfo/DragonInfo/MagicInfo 相关跳过或读取逻辑
 ```
 
 **执行步骤**：
 
 1. 在 `world/map` 下新建 `mirdb` 目录和 `mod.rs`，将现有 `mirdb.rs` 内容整体迁入，保持对外函数签名不变。
 2. 逐步将内部 `read_*`/`skip_*` 函数按上面建议拆进对应子模块中，并在 `mod.rs` 中 `pub(super)` 暴露必要 API。
 3. 保持 `load_*_from_mirdb` 始终位于 `mod.rs` 中，作为统一入口，减少外部调用方改动。
 4. 对照 C# 源码，边拆分边增加注释和错误信息，确保解析偏移行为保持一致。
 
 #### 7.3.4 其他协议类大文件（`login.rs` / `npc.rs` / `magic.rs` / `item.rs`）
 
 **关联子计划**：子计划 1、子计划 3、子计划 4（高级系统）。
 
 这些文件主要由大量 `S*`/`C*` 协议结构体 + encode/decode 组成，复杂度相对集中。
 当前不强制立刻拆分，但建议在相关子计划推进时顺带整理：
 
 1. **`crystal-shared-proto/src/login.rs`**（约 1092 行）
    - 角色：定义 `ClientPacketId` / `ServerPacketId` 枚举，以及登录/连接阶段基础 C*/S* 报文。
    - 建议在未来将“包 ID 枚举”和“具体登录报文结构”拆到两个文件：
      - `ids.rs`：只存放 `ClientPacketId` / `ServerPacketId`；
      - `login_packets.rs`：存放 `CLogin` / `SLogin` / `CNewAccount` / `SNewAccount` 等。
    - 拆分时需确保所有使用枚举的地方更新引用路径。
 
 2. **`crystal-shared-proto/src/npc.rs`**（约 1056 行）
    - 角色：NPC 外观、NPC 对话/交易/觉醒等多套系统的协议合集。
    - 建议按系统拆分：
      - `npc_base.rs`：`SObjectNpc` / `SNpcResponse` 等基础展示与对话；
      - `npc_trade.rs`：`SNpcGoods` / `SNpcSell` / `SNpcRepair` / `SNpcsRepair` 等；
      - `npc_awakening.rs`：觉醒/分解/降级/重置类协议；
      - `npc_market.rs`：拍卖行/市场相关协议。
    - 通过 `mod.rs` 统一 `pub use`，对外继续暴露在 `crate::npc` 命名空间下。
 
 3. **`crystal-shared-proto/src/magic.rs`**（约 856 行）
    - 角色：技能/魔法相关协议及大量 roundtrip 测试。
    - 建议优先将测试迁移到单独的 `tests` 模块或文件，减小主文件体积：
      - 如 `magic.rs` 只保留类型与实现，`magic_tests.rs` 存放单元测试并在 `mod.rs` 中 `#[cfg(test)] mod magic_tests;`。
 
 4. **`crystal-shared-proto/src/item.rs`**（约 627 行）
    - 角色：物品信息、新物品、刷新、耐久变化、仓库/背包扩展等。
    - 根据子计划 3 进度，考虑在实现完整 `ItemInfoData` / `UserItemData` 流程时：
      - 将“结构化 roundtrip 测试”迁出到独立测试模块；
      - 若需要，可再按“展示类协议”（如 `SNewItemInfo`/`SNewChatItem`）与“状态变化协议”（如 `SDuraChanged`/`SDeleteItem`）拆成两个文件。
 
 ### 7.4 拆分实施顺序建议
 
 - **第一阶段（优先，配合子计划 1）**：
   - 完成 `connection/handler.rs` 的阶段化拆分；
   - 完成 `user.rs` → `user/` 目录模块重构。
 
 - **第二阶段（配合子计划 2/3）**：
   - 将 `mirdb.rs` 重构为 `mirdb/` 目录模块，方便地图/物品/怪物等解析代码的并行演进。
 
 - **第三阶段（视需要进行）**：
   - 根据协议扩展情况，对 `login.rs` / `npc.rs` / `magic.rs` / `item.rs` 进行渐进式拆分，
   - 优先迁移测试代码，减少主协议文件体积.

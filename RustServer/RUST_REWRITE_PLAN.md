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

**当前进度概览（2025-12-01 更新）**
- **已完成**：
  - 登录 / 注册 / 改密 / 角色列表 / 新建 / 删除 / StartGame / LogOut 等基础流转已在 `crystal-server-bin/src/connection` 中实现，并拆分为 `login_stage`, `select_stage` 以及按领域划分的子模块（`session`, `visibility`, `movement`, `npc`, `chat`, `gm_commands`, `guild`, `map`, `market`, `item` 等）；原 `ingame_stage.rs` 已拆空并删除。
  - `ClientPacketId` / `RawPacket` 解码和大部分核心 C* / S* 包已在 `crystal-shared-proto` 中实现，ID 数值与 C# 完全一致。
  - 世界命令对接（StartGame / Turn / Walk / Run / Attack / PickUp / DropItem 等）已通过 `world::WorldCommand` 与 `World::handle_command` 路由打通。
  - 基础战斗命令（Attack）已连接到 `combat` 模块。
  - `handler.rs` 大文件拆分已完成。
  - 组队相关协议（`SwitchGroup`, `AddMember`, `DellMember`, `GroupInvite`）已在 `crystal-server-bin/src/connection/group.rs` 中对接 `WorldCommand`，基础行为对齐 C#。
  - 账号存储层：在 `crystal-server-core::account` / `crystal-server-db` 中扩展 `accounts` 表（`banned` / `ban_reason` / `ban_expires_at` / `require_password_change` / `wrong_password_count`），并通过 `AccountStatus` 与 `AccountStore::load_account_status` / `save_account_status` 在登录 / 改密流程中复现了 C# `AccountInfo` / `Envir.Login` / `Envir.ChangePassword` 的封禁与 `RequirePasswordChange` 语义。
  - NPC 买卖与 GameShop：在 `connection::market` / `connection::gameshop` 中实现 NPC 商店买卖与 BuyBack 列表，使用脚本 [TRADE] 配置物品。
  - 玩家交易系统：`connection::trade` 与 `world::World` 支持交易邀请 / 锁定 / 金币与物品交换 / 取消回滚，并做基础的容量与金币上限检查。
  - 邮件系统协议：`connection::mail` + `crystal-server-core::account::StoredMail` + `world::configs::mail_config` 完成邮件收发、附件/金币寄送与邮资计算，并在收件人在线时推送 `SReceiveMail`。
  - 行会扩展协议：在 `connection::guild` 中实现行会仓库物品/金币操作、行会经验广播与 `SGuildStatus` 刷新，桥接 `world::GuildManager` / `guild_settings`。

- **待办事项**：
  - **缺失的协议处理**：全局 `Market` 拍卖行相关 C* 包（`ConsignItem`, `MarketSearch`, `MarketRefresh`, `MarketPage`, `MarketBuy`, `MarketGetBack`, `MarketSellNow`）、`Quest` (任务)、`Harvest` (采集) 以及与任务/市场相关的部分 NPC 指令。
  - **IP 封禁策略**：已在传输层（`crystal-server-net::run_server` + `ServerConfig.max_ip` / `ip_block_seconds`）实现基于 IP 的最大并发连接数限制与短期封禁；基于登录失败/异常包/账号创建次数等的 IP 封禁与解封逻辑仍待迁移。
  - **边界情况处理**：对照 C# 完善错误码返回和异常流程保护，并为 Trade / Mail / NPC 商店等复杂协议补充日志与测试用例。

### 主要涉及代码

- **C# 参考**
  - `Server/MirNetwork/MirConnection.cs`
  - `Server/Settings.cs`（`Network` 部分配置）
  - ClientPackets/ServerPackets 定义（包 ID 与结构）

- **Rust 目标位置（仅限 RustServer 目录）**
  - `crystal-shared-proto`：协议定义。
  - `crystal-server-net`：传输层。
  - `crystal-server-bin/src/connection/*.rs`：连接状态机与包分发。

### 接下来计划

- **1. 补齐剩余协议与分发**
  - 在各 `connection` 子模块中（`npc.rs`, `guild.rs`, `item.rs`, `map.rs`, `market.rs`, `quest.rs` 等）补齐全局 `Market`(拍卖行)、`Quest`、`Harvest` 等高级玩法相关包的处理分支，并将其路由到 `world::WorldCommand` / Admin 模块。
  - 对暂时没有完整业务逻辑的部分，先返回明确错误/提示，并记录日志，避免 silent fail。

- **2. IP 封禁与安全策略**
  - 在现有传输层 `max_ip` / `ip_block_seconds` 基础上，参考 C# `Envir.UpdateIPBlock` 行为，实现登录失败次数统计、异常包/超量包、过多新建账号/角色等触发的 IP 封禁/解封逻辑。
  - 在 Admin 或内部控制命令中预留“查看/解除封禁”能力（与高级系统子计划协同）。

- **3. 边界情况与错误码对齐**
  - 登录 / 注册 / 改密 流程的错误码与封禁语义（账号不存在 / 密码错误 / 版本不符 / LoginBanned / ChangePasswordBanned / RequirePasswordChange）已在 Rust 侧基本对齐 C# `MirConnection` / `Envir`。
  - 其余协议（如 StartGame / 删除角色 / 未来的 Market / Trade / Quest 等）的错误分支与结构化日志仍需按 C# 行为补齐。

---

## 三、子计划2：世界时间 & 地图系统重写

### 目标

- 在 Rust 中实现等价 C# `Envir` 的世界时间与主循环：
  - 驱动定时器（刷怪、清理、buff 等）。
- 完整迁移 `Map` 地图系统：
  - 地图加载、判定、物品/怪物管理。

**当前进度概览（2025-12-01 更新）**
- **已完成**：
  - `world::World` 具备玩家、地图、怪物、物品管理能力。
  - `world::map::load_map_from_file` 实现地图加载与可行走判定。
  - **主循环 Tick**：在 `crystal-server-core/src/world/monster_runtime.rs` 的 `update` 方法中实现了时间驱动、Respawn 计数更新、怪物 AI 驱动（Search/Roam/Chase/Attack）。
  - 怪物刷新逻辑（Respawn）已移植，支持 `Delay`, `RandomDelay`, `RespawnTicks`.
  - **MapItem 过期清理**：通过 `MapItem.expire_time_ms` 与 `World::process_map_items` 周期性移除过期掉落物，行为对齐 C#。
  - 玩家 Buff 到期清理与属性重算：`World::update` 中的 `process_player_buffs` 会根据 `expire_time_ms` 移除过期 Buff、重算 Buff 属性，并发出 `WorldEvent::RemoveBuff` / `WorldEvent::SpellToggle`.
  - 安全区 HP 恢复：当 `WorldConfig.safe_zone_healing` 启用时，`World::update` 通过 `process_safezone_healing` 为安全区内玩家定期回复 HP。
  - 自然 HP/MP 恢复：`World::update` 中的 `process_player_regen` 每 10 秒驱动玩家 HP/MP 自然恢复，使用 `HealthRecovery` / `SpellRecovery` 影响回复量。

- **待办事项**：
  - **Buff 系统语义补全**：对齐 C# Buff 系统，补齐 Buff 暂停/恢复、叠加策略，以及怪物 Buff 的 tick 驱动。
  - **环境更新**：天气/时间（Day/Night）更新逻辑。

### 主要涉及代码

- **Rust 目标位置**
  - `crystal-server-core::world::monster_runtime.rs`：主循环与怪物运行时。
  - `crystal-server-core::world::world.rs`：世界状态容器。

### 接下来计划

- **1. Buff / 状态系统语义补全**
  - 在现有 `process_player_buffs` / BuffInfo 表的基础上，对齐 C# Buff 系统的叠加/冲突/暂停规则（`PauseInSafeZone`, `RemoveOnDeath`, `Debuff` Tick 等），并补齐怪物 Buff 的 Tick 行为与属性刷新。

- **2. 世界环境系统**
  - 实现 Day/Night 切换及相关通知/可视效果（若客户端有对应支持），并预留天气/特殊事件扩展点。

---

## 四、子计划3：玩家 / 物品 / 基础战斗逻辑

### 目标

- 迁移 C# `PlayerObject` 核心逻辑。
- 物品系统与基础交互。
- 战斗闭环（物理/魔法）。

**当前进度概览（2025-11-28 更新）**
- **已完成**：
  - `PlayerState` 及属性/背包/装备系统。
  - 物品移动/穿脱/使用（药水/回城卷/随机卷）已实现。
  - **掉落与拾取**：`DropItem` / `PickUp` 已实现，支持金币与物品。
  - **基础战斗**：
    - 物理攻击（Player vs Monster）已实现，包含命中/暴击/防御计算。
    - 怪物反击（Monster vs Player）已实现。
    - 死亡与经验获取已实现。
    - `FatalSword` 技能逻辑已作为样例实现。
  - **玩家交易系统**：在 `connection::trade` 与 `world::World` 中实现玩家间交易邀请 / 锁定 / 金币与物品交换 / 取消回滚，并复用了现有背包/装备/`MapItem` 结构。

- **待办事项**：
  - **魔法/技能系统**：除 `FatalSword` 以及部分 Warrior 技能（如 `FlamingSword`、`Rage`、`ImmortalSkin`、`CounterAttack`、`HalfMoon`/`CrossHalfMoon` 等）的基础实现外，大部分技能仍未完全迁移/对齐（如火球术、治愈术等）。
  - **玩家死亡与复活**：虽然有 `SDeath`，但完整的死亡惩罚（掉装备）、复活（原地/回城）流程需完善。
  - **交易系统边界与风控**：对照 C# `PlayerObject.Trade` 回归断线/越距/背包不足等场景，补齐日志、防刷与异常恢复策略。
  - **组队系统**：经验共享逻辑（基础 AllowGroup/邀请/入队/踢人 已在 `world::World` / `connection::group` 中实现）。

### 接下来计划

1. **技能系统子计划（3.1，可与 3.2/3.3/3.4 并行）**
   - 3.1.1：对齐 `MagicInfo` / `UserMagic` 数据与学习/升级/快捷键逻辑，保证 `magic_damage` 行为与 C# 等价。
   - 3.1.2：实现单体攻击类技能（如 `FireBall` / `SoulFireBall`），完善射程、目标选取与伤害计算，对照 C# `PlayerObject`.
   - 3.1.3：补完 Rage / ImmortalSkin / CounterAttack / FlamingSword 等 Buff 类技能在服务器端的 Buff 叠加与 `SAddBuff` / `SpellToggle` 协议.

2. **玩家死亡与复活子计划（3.2，可与 3.1/3.3/3.4 并行）**
   - 3.2.1：对照 C# `PlayerObject.Die` / `RedDeathDrop` / `DeathDrop`，实现玩家死亡时的红名/普通掉落逻辑（含 BindMode、Rental、婚戒等判断）。
   - 3.2.2：接入 `revive_player_in_place` / `revive_player_to_position`，完成原地/回城/绑定点复活流程与 `SRevived` / `SObjectRevived` 协议.

3. **交易系统回归与安全策略子计划（3.3，可与 3.1/3.2/3.4 并行）**
   - 3.3.1：基于现有 `connection::trade` / `world::World` 实现，对照 C# 场景编写集成测试（断线、取消、背包不足、金币上限等）。
   - 3.3.2：补齐防刷逻辑与结构化日志，必要时暴露 Admin/GM 监控入口。

4. **组队经验子计划（3.4，仅依赖已存在的组队基础逻辑）**
   - 3.4.1：在 `world::World` 中实现组队经验分配函数，对照 C# `PlayerObject.WinExp` 的组队加成和范围判定.
   - 3.4.2：将 `WorldEvent::GainExperience` 接入组队分配逻辑，单人视为 1 人队伍.
   - 3.4.3：通过日志或 GM 工具，对同一场景下 C# / Rust 经验数值进行抽样比对.

---

## 五、子计划4：高级系统 & 运维/Admin 整合

### 目标

- 迁移行会、攻城战、任务、排行榜。
- 完善 Admin 控制台。

**当前进度概览**
- **已完成**：
  - Admin HTTP 服务框架与基础 Metrics 接口。
  - 行会系统基础与扩展：`GuildManager`、创建行会、`CGuildInvite` 处理，以及行会仓库物品/金币操作、行会经验获取与 `SGuildStatus` 广播，配置来源于 `guild_settings`。
  - 邮件系统：`connection::mail` + `StoredMail` + `mail_config` 已支持角色邮箱加载、邮件收发、金币与物品附件、邮资计算及在线收件推送。

- **待办事项**：
  - 完整的行会高级功能（成员管理 UI 同步、战争、行会等级 Buff、排行榜等）。
  - 攻城战系统（Conquest）及相关地图/NPC 逻辑。
  - 任务系统（NPC 对话与玩家任务状态），将 `QuestInfo` / `QuestManager` 接入世界加载与持久化。
  - 市场/拍卖行（全局寄售 Market，区别于已实现的 NPC 商店/GameShop）。

### 接下来计划

1. **任务系统**
   - 基于 `crystal-server-core::quest::QuestInfo` / `QuestManager` 实现 QuestInfo 解析与世界加载。
   - 设计玩家任务状态结构（`PlayerQuest`），并将 NPC 对话中的任务指令/脚本对接到 Rust 世界与连接层。

2. **完善行会与攻城**
   - 移植行会战争与沙巴克攻城逻辑，对接 `conquest.rs` 数据结构与 `GuildManager`。

3. **市场 / 拍卖行**
   - 设计全局 Market（拍卖行）数据结构与持久化方案，补齐 `ConsignItem` / 一系列 `Market*` C* 包处理。
   - 与现有 NPC 商店/GameShop 区分清晰，确保价格/刷金防护策略与 C# 一致。

---

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

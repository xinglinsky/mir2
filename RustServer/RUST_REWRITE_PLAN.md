# Mir2 RustServer 重写总体计划

> 本计划遵循 `RUST_SERVER_RULES`（行为与 C# 保持一致、仅修改 RustServer 目录、错误处理避免 panic、单文件不超过 800 行并按职责拆分模块等）。

## 〇、进度口径与现状核验（2025-12-14）

为避免“看起来实现了，但实际客户端走不到”的情况，本计划后续统一按以下口径描述进度：

- **Implemented（已实现）**：Rust 侧已有对应数据结构/核心逻辑代码。
- **Wired（已接入）**：连接层 `crystal-server-bin/src/connection/handler.rs` 已把对应 `ClientPacketId` 分发到具体 handler，客户端实际可触发。
- **Verified（已验证）**：与 C# 服务端对照验证通过（至少包含：协议字段一致、关键边界一致、可复现的回归用例）。

**当前代码事实（必须以此为准）**：

- **连接层唯一分发入口**：当前 Rust 服网络入口使用 `LoginConnection`，其包分发由 `crystal-server-bin/src/connection/handler.rs` 的 `match ClientPacketId` 决定。
- **已确认 Stub（Wired 但未实现业务）**：`Harvest`、全套 `Market*`/`ConsignItem`、`GuildWarReturn` 等目前只返回“未实现”的系统消息。
- **已确认缺失（未接入 / Unhandled）**：大量 C# `MirConnection.ProcessPacket` 中存在的包在 Rust `handler.rs` 中没有分支（例如 `MergeItem/SplitItem/Refine*`、`DropGold`、`RangeAttack`、`SpellToggle`、`Fishing*/Awakening*`、`Marriage/Mentor`、`IntelligentCreature*`、`Rental*`、`Opendoor` 等）。这些功能即使在 core/world 中已有部分代码，也需要先完成 **Wired** 才能进入“可玩/可测”。

---


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

**当前进度概览（2025-12-14 更新，口径：Implemented/Wired/Verified）**

### 2.1 已接入（Wired）的协议（客户端可触发）

- **账号/选角流程（Wired，部分 Verified）**：
  - `ClientVersion` / `NewAccount` / `Login` / `ChangePassword` / `NewCharacter` / `DeleteCharacter` / `StartGame` / `LogOut`。
- **基础移动与战斗闭环（Wired，需系统回归 Verified）**：
  - `Turn` / `Walk` / `Run` / `Attack` / `Magic` / `MagicKey`。
  - `PickUp` / `DropItem`。
- **NPC 与地图相关（Wired）**：
  - `CallNPC` / `BuyItem` / `SellItem` / `CraftItem`。
  - `RequestMapInfo` / `TeleportToNPC` / `SearchMap`。
- **行会/组队/交易/邮件/好友（Wired，边界需回归 Verified）**：
  - 组队：`SwitchGroup` / `AddMember` / `DellMember` / `GroupInvite`。
  - 交易：`TradeRequest` / `TradeReply` / `TradeGold` / `TradeConfirm` / `TradeCancel` / `DepositTradeItem` / `RetrieveTradeItem`。
  - 邮件：`SendMail` / `ReadMail` / `CollectParcel` / `DeleteMail` / `LockMail` / `MailLockedItem` / `MailCost`。
  - 好友：`AddFriend` / `RemoveFriend` / `RefreshFriends` / `AddMemo`。
  - 行会：`EditGuildMember` / `EditGuildNotice` / `GuildInvite` / `GuildNameReturn` / `RequestGuildInfo` / `GuildStorageGoldChange` / `GuildStorageItemChange`。
- **其他（Wired）**：
  - `KeepAlive` / `Disconnect`。
  - `TownRevive`（复活流程尚需对齐 C# 细节 Verified）。
  - `AcceptQuest` / `FinishQuest` / `AbandonQuest` / `ShareQuest`（当前奖励发放与队伍共享行为为最小实现，需补齐 Verified）。
  - `AcceptReincarnation` / `CancelReincarnation`。
  - `GetRanking` / `GameshopBuy`。
  - `Inspect` / `Observe` / `RequestUserName` / `RequestChatItem`（Wired；功能为最小实现，需按 UI 路径回归 Verified）。
  - Hero UI：`NewHero` / `SetHeroBehaviour` / `ChangeHero` / `TakeBackHeroItem` / `TransferHeroItem`（Wired；Manage/Create/召唤/背包拖拽等需系统回归 Verified）。
  - 普通聊天（Normal Chat）已改为以 `SObjectChat` 回显并向附近玩家广播（Wired；需回归 Verified）。

### 2.2 已接入但仍为 Stub（Wired 但未实现业务）

- `Harvest`。
- 全套拍卖行：`ConsignItem` / `MarketSearch` / `MarketRefresh` / `MarketPage` / `MarketBuy` / `MarketGetBack` / `MarketSellNow`。
- `GuildWarReturn`。

### 2.3 目前缺失的协议（未接入 / Unhandled，优先级极高）

以下在 C# `MirConnection.ProcessPacket` 中存在，但在 Rust `handler.rs` 中暂未分发：

- **物品/锻造/精炼链路**：`MergeItem` / `SplitItem` / `RemoveSlotItem` / `DropGold` / `DepositRefineItem` / `RetrieveRefineItem` / `RefineCancel` / `RefineItem` / `CheckRefine` / `ReplaceWedRing` / `BuyItemBack` / `RepairItem` / `SRepairItem` / `EquipSlotItem` / `CombineItem`。
- **战斗补全**：`RangeAttack` / `ChangeTrade` / `SpellToggle`。
- **英雄/钓鱼/觉醒/宠物智能体**：`FishingCast` / `FishingChangeAutocast` / `AwakeningNeedMaterials` / `AwakeningLockedItem` / `Awakening` / `DisassembleItem` / `DowngradeAwakening` / `ResetAddedItem` / `UpdateIntelligentCreature` / `IntelligentCreaturePickup` / `RequestIntelligentCreatureUpdates`。
- **社交扩展**：`MarriageRequest` / `MarriageReply` / `ChangeMarriage` / `DivorceRequest` / `DivorceReply` / `AddMentor` / `MentorReply` / `AllowMentor` / `CancelMentor`。
- **运维与杂项**：`GuildBuffUpdate` / `NPCConfirmInput` / `ReportIssue` / `Opendoor` / `GetRentedItems` / 全套 `ItemRental*` / `GuildTerritoryPage` / `PurchaseGuildTerritory`。

> 说明：`Inspect/Observe/RequestUserName/RequestChatItem` 与 Hero UI (`NewHero/SetHeroBehaviour/ChangeHero/TransferHeroItem/TakeBackHeroItem`) 已完成 Wired，因此从“缺失清单”中移除；但仍需要以客户端 UI 路径进行 Verified 回归。

> 说明：上述缺失项会直接影响“严格对齐 C#”这一目标，因为客户端路径与服务器状态机不完整。子计划 1 的下一阶段应首先把这些包处理补齐到 **至少 Wired**。

### 2.4 IP 封禁与限流（需对齐 C# 行为）

- 传输层已存在 `max_ip` / `ip_block_seconds` 级别的限制，但仍需对齐 C# `Envir.UpdateIPBlock`：
  - 登录失败次数与周期重置；
  - 异常包（解码失败、超包量、越权操作）的封禁与日志；
  - 管理端解封与查询。

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

1. **协议全量对齐（优先级最高，目标：覆盖 C# `MirConnection.ProcessPacket`）**
   - 1.1：补齐并接入“物品/锻造/精炼链路”相关包（`MergeItem/SplitItem/Refine*`、`DropGold`、`BuyItemBack/Repair*` 等），并以 C# 为准实现：
     - 栈合并/拆分与背包容量边界；
     - 金币上限/溢出处理；
     - NPC 交互距离与 NPCPage 校验；
     - 各类失败原因的返回包/提示（**中文本土化**，但语义必须等价）。
   - 1.2：补齐战斗相关未覆盖包（`RangeAttack/SpellToggle/ChangeTrade`）并对齐 C# 触发条件。
   - 1.3：补齐“钓鱼/觉醒/智能宠物”等系统的协议接入与最小可用闭环（先 Wired，再完善业务）。
   - 1.4：补齐“运维与杂项”协议（`Opendoor`、`Rental*`、`GuildTerritory*`、`ReportIssue` 等）。

2. **一致性验证体系（目标：让“严格一致”可量化）**
   - 2.1：建立 **Packet 覆盖矩阵**（C# packet → Rust handler → world command/event），每个条目必须标注 Implemented/Wired/Verified。
   - 2.2：引入 **Record/Replay**：
     - 从客户端侧记录 `ClientPacketId` 序列与 payload（或从 C# 服务端日志/抓包导出），在 Rust 侧回放并对比关键输出包序列与世界状态摘要。
   - 2.3：关键公式与随机一致性：实现并使用 **.NET Random 等价实现**（避免 `rand` 与 C# `Random` 分布差异导致掉落/伤害/寻路随机不一致）。
   - 2.4：为高风险路径补齐自动化用例：交易、邮件、拍卖行、复活/掉落、任务奖励、封禁策略。

3. **性能最优实现策略（前提：不改变语义）**
   - 3.1：降低 `world: Mutex<World>` 的锁竞争：将连接线程改为只做解码/校验，把 `WorldCommand` 通过队列投递给世界线程处理（Actor/Command Queue），输出 `WorldEvent` 再异步分发。
   - 3.2：避免热路径重复 IO：地图文件加载应缓存（例如在 WorldDatabase 或 World 内维护 Map 缓存），禁止在技能/传送路径中频繁 `load_map_from_file`。
   - 3.3：热点数据结构优化：可见列表/范围查询（`sessions_in_range_for_map`）按地图建立空间索引（网格桶/分块），减少 O(n) 扫描。
   - 3.4：日志与指标：保持 `tracing` 结构化日志，但在热路径避免过多 `debug!`，使用采样或按模块开关。

4. **消息通知中文本土化（目标：统一、可维护、可审计）**
   - 4.1：禁止在业务代码中散落硬编码字符串；统一使用 `i18n`/`msg` 模块：`msg(key, args...) -> String`。
   - 4.2：支持从配置加载中文语言表（建议兼容 `Configs/Language.ini` 或 RustServer 自有 `i18n/zh-CN.toml`），并保留 fallback。
   - 4.3：对标 C# 系统消息语义：
     - 文本可中文化，但触发时机、错误码、断线原因必须与 C# 一致；
     - 对外可见的 GM/系统广播也必须走同一套本土化入口，避免出现“部分中文、部分英文、部分硬编码”。

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

- 迁移 C# `PlayerObject` 核心逻辑.
- 物品系统与基础交互.
- 战斗闭环（物理/魔法）。

**当前进度概览（2025-12-06 更新）**
- **已完成**：
  - `PlayerState` 及属性/背包/装备系统；角色登陆后背包、装备、魔法列表、攻击/宠物模式等均可在 Rust 服中正确加载与保存.
  - 物品移动/穿脱/使用（药水/回城卷/随机卷）已实现，并通过 `connection::item` / `world::World` 对应 C# 行为.
  - **掉落与拾取**：`DropItem` / `PickUp` 已实现，支持金币与物品，并走统一的 `MapItem` 过期清理与可见性管线.
  - **基础战斗**：
    - 物理攻击（Player vs Monster）已实现，包含命中/暴击/防御计算，落地到 `world::combat`；
    - 怪物反击（Monster vs Player）已实现，使用统一的 `MonsterHitPlayer` / `ObjectStruck` 世界事件与 SObjectStruck/SDamageIndicator；
    - 经验获取与升级：`World::gain_experience_for_session`、`WorldEvent::GainExperience` / `PlayerLevelChanged` 已接入，客户端通过 `SGainExperience` / `SLevelChanged` / `SObjectLeveled` 与 C# 行为对齐；
    - 玩家死亡后的死亡包与基础死亡状态已实现，服务端会发送 `SObjectDied` 等场景包.
  - **战斗相关被动与 Buff/毒**：
    - Warrior：Slaying/Fencing/SpiritSword 被动通过 `recompute_player_passives_for_session` 影响 Accuracy/MaxDC；
    - Assassin：MoonLight/DarkBody 开场一击加成、SwiftFeet/Haste 等 Buff 已在 `world::skills::assassin` 中实现，并通过 Buff 系统下发；
    - Taoist/Wizard：SoulShield/BlessedArmour/UltimateEnhancer/EnergyShield 等 Buff 技能已接入 `add_player_buff` / `WorldEvent::AddBuff` / `SpellToggle`；
    - 绿色/红色毒（Poisoning）与怪物来源毒均已通过 `PoisonInstance` 管理，玩家/怪物的毒 Tick 与叠加/抵抗规则在 `world::combat` / `monster_runtime` 中实现，并映射到 `S.Poisoned` / `S.ObjectPoisoned`.
  - **魔法/技能系统（进展）**：
    - 共用的 `compute_pure_magic_attack_damage`、`compute_magic_mana_cost`、`check_and_update_magic_cooldown`、`level_up_magic_for_player` 已在 `world::skills` / `world::world` 中实现，对齐 `MagicInfo` / `UserMagic` 数据与耗蓝/冷却/熟练度逻辑；
    - Warrior：`FlamingSword`、`Rage`、`ImmortalSkin`、`CounterAttack`、`Fury`、`HalfMoon` / `CrossHalfMoon` / `Thrusting` 等核心技能在 `world::skills::warrior` 与 `world::combat` 中完成迁移，含掉落/经验归属与红毒护甲减免等细节；
    - Wizard：单体与 AoE 魔法（`FireBall`/`GreatFireBall`/`SoulFireBall`/`ThunderBolt` 以及 `FireBang`/`IceStorm`、`ThunderStorm`/`FlameField`、`Blizzard`、`Lightning`、`HellFire`、`FireWall` 等）已实现完整的伤害、命中范围与 `SMagic`/`SObjectMagic` 可视事件；
    - Taoist：`Healing`/`MassHealing`、`Poisoning`、`SoulShield`/`BlessedArmour`、`EnergyShield`、`UltimateEnhancer` 以及 Skeleton/Shinsu/HolyDeva 等召唤与宠物模式逻辑已在 `world::skills::taoist` 中落地；
    - Assassin：MoonLight/DarkBody、SwiftFeet、Haste 等核心技能已接入战斗与 Buff 管线；
    - 魔法升级与延迟：`WorldEvent::MagicLeveled` / `MagicDelay` / `MagicCast` 均通过 `connection::movement::handle_world_events` 映射为 `SMagicLeveled` / `SMagicDelay` / `SMagicCast`，保持客户端技能栏冷却/熟练度表现一致.
  - **玩家交易系统**：在 `connection::trade` 与 `world::World` 中实现玩家间交易邀请 / 回复 / 锁定 / 确认 / 取消回滚，覆盖金币与物品交换，并做基础容量与金币上限检查.

- **待办事项**：
  - **技能覆盖与数值对齐**：
    - 个别高级或罕见技能（如部分弓手技能、少数怪物/宠物专用法术）的数值与范围仍需对照 C# 回归；
    - 需要增加系统化的 Record/Replay 或脚本化战斗用例，对 FireBall/ThunderBolt/Blizzard/HellFire 等关键技能的伤害分布与命中行为做批量比对.
  - **玩家死亡与复活**：
    - `world::combat` 已实现玩家死亡掉落（普通/红名）、BindMode/Rental/婚戒/Spirit 套等规则的大部分语义，并通过 `SObjectDied`/地面掉落包驱动客户端表现；
    - 原地复活/回城/绑定点复活的完整流程（含 `SRevived` / `SObjectRevived`、复活点选择与死亡惩罚 UI 提示）仍需按 C# `PlayerObject.Die` / `RedDeathDrop` / `TownRevive` / 复活道具行为补齐，并接入 `revive_player_in_place` / `revive_player_to_position` 等世界辅助函数.
  - **交易系统边界与风控**：对照 C# `PlayerObject.Trade` 补齐断线、越距、背包不足、金币上限、租赁/绑定物品等场景下的防刷逻辑与结构化日志，并视情况暴露 Admin/GM 监控入口.
  - **组队系统**：
    - 组队基础逻辑（AllowGroup / 邀请 / 入队 / 踢人）已经在 `world::World` / `connection::group` 中实现；
    - 组队经验共享与加成仍按单人经验发放，需要实现 `PlayerObject.WinExp` 等价逻辑，将 `WorldEvent::GainExperience` 接入组队分配，并通过日志/GM 工具抽样对比 C# 与 Rust 数值.

### 接下来计划

1. **技能系统子计划（3.1，可与 3.2/3.3/3.4 并行）**
   - 3.1.1：对齐 `MagicInfo` / `UserMagic` 数据与学习/升级/快捷键逻辑，保证 `magic_damage` 行为与 C# 等价.
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

## 七、下一步详细计划（建议按 1-2 周冲刺执行）

> 目标：优先把“已 Wired 的 UI 路径”做成 Verified，避免后续在物品/战斗等大改动中引入回归；同时推进下一批高频基础包到 Wired。

### Sprint A：UI 回归与稳定性（优先级 P0）

- **A1：普通聊天可用性回归（Verified）**
  - 验收：
    - 自己发送普通聊天，聊天窗口可见（回显）。
    - 同地图同视野范围内其他玩家可见（广播）。
  - 覆盖：Normal、私聊、GM 指令不影响普通聊天。

- **A2：聊天物品链接（Verified）**
  - 验收：
    - 发送带链接物品的消息后，点击链接客户端能请求 `RequestChatItem` 并收到 `SChatItemStats`。
    - 物品展示信息完整且不会崩溃。
  - 风险点：缓存命中/失效、唯一 ID 对齐。

- **A3：Inspect/Observe（Verified）**
  - 验收：
    - Inspect 面板能正常打开并显示基本信息。
    - Observe：目标允许观察时可传送；关闭观察时有明确系统提示。
  - 覆盖：
    - `allow_observe` 开关（`@ALLOWOBSERVE`）与服务端配置 `observe.allow_observe`。

- **A4：Hero 管理与创建全链路（Verified）**
  - 验收：
    - `[@MANAGEHERO]` 能打开 HeroManageDialog。
    - `[@CREATEHERO]` 能打开创建界面；创建成功后列表刷新；切换英雄不会崩。
    - 召唤/收回英雄、英雄背包与玩家背包拖拽维持可用。
  - 风险点：ClientHeroInformation 字段序列化、列表 index/off-by-one。

### Sprint B：物品/金币/修理/回购（Wired + 最小业务，优先级 P0）

- **B1：金币相关**
  - `DropGold`（Wired + 最小实现）：金币扣除、地面掉落可拾取、上限检查。

- **B2：栈合并/拆分**
  - `MergeItem` / `SplitItem`：背包容量/空格、最大堆叠、绑定/特殊物品限制对齐 C#。

- **B3：修理/回购链路**
  - `RepairItem` / `BuyItemBack`：NPC 距离与页面校验、费用计算、失败提示一致。

### Sprint C：战斗补全（Wired + 对齐触发条件，优先级 P1）

- **C1：RangeAttack**
  - 对齐弓/远程相关触发与弹道/命中反馈包。

- **C2：SpellToggle / ChangeTrade**
  - 补齐客户端技能开关与交易状态变更相关逻辑，避免 UI 卡死。

### Sprint D：任务系统深化（P1，依赖 Sprint A 稳定后推进）

- **D1：Quest 业务补齐**
  - `AcceptQuest/FinishQuest/AbandonQuest/ShareQuest`：补齐奖励发放、组队共享距离判定、选奖励物品索引处理。
  - 验收：典型新手任务可完整接取→完成→领取奖励，UI 任务列表状态更新正确。

### Sprint E：拍卖行/市场（P2，先从 Stub → 最小可用）

- **E1：Market 系列包**
  - 先完成协议 Wired（避免 UI 点按钮无响应），再逐步补齐寄售/购买/下架/找回。

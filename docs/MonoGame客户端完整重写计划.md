# MonoGame 客户端完整重写计划（Windows x64 优先，预留 Mac；KTX2 单包；点地寻路）

## 0. 背景与目标

### 0.1 当前 C# 客户端架构（现状摘要）

- **窗口/主循环**：WinForms + `SlimDX.Direct3D9`，`Application.Idle` 自旋主循环。
- **渲染**：`DXManager` 管 D3D9 Device/Sprite/Shader；`MLibrary` 自定义 `.Lib` 图库按需加载。
- **UI**：自研 `MirControl/MirScene` 控件树，部分控件使用 RenderTarget 纹理缓存。
- **网络**：`TcpClient BeginReceive/BeginSend + ConcurrentQueue`，包处理与渲染同线程。

### 0.2 重写目标

- **稳定性**：消除 D3D9/SlimDX、DeviceLost 等不稳定因素；网络与渲染解耦，避免长帧。
- **性能**：2D 精灵/地图批处理，减少 draw call；资源加载与解码可控；减少 GC 压力。
- **可维护性**：模块分层清晰，可测试、可观测（指标/日志）。
- **跨平台预留**：Windows x64 先上线，后续可上 Mac（同一套代码 + 资源包）。
- **UI 风格**：保持现有 Mir 风格（布局/交互/皮肤），但实现更稳更快。

### 0.3 已确定关键决策（本计划的“硬约束”）

- **MVP 先做**：登录 + 进图 + 走动 + 聊天。
- **移动输入方式**：**鼠标点地寻路（客户端寻路）**。
- **移动协议确认**：服务端接收 **方向步进**（`CWalk/CRun { direction }`），不是目标点。
- **资源置换**：统一使用 **KTX2（BasisU）单包策略**（同一份内容包跨 Windows/Mac）。

---

## 1. 技术选型

### 1.1 渲染与运行时

- **MonoGame 运行目标**：`DesktopGL`（优先）。
  - 理由：跨 Windows/Mac 迁移成本最低，现代图形后端更稳定。
- **.NET**：.NET 8。

### 1.2 资源格式

- **统一容器**：`content.cpack`（单文件 pack：index + offset + sha256）。
- **纹理**：KTX2（Basis Universal）。
  - **MVP 阶段**：允许 CPU 解码为 RGBA 上传 `Texture2D`（先跑通）。
  - **性能阶段**：接入 GPU/平台转码：Windows→BCn，Mac→ASTC（同一 KTX2 数据源）。

### 1.3 网络与协议

- **协议兼容**：优先复用现有 `Shared`/包定义（字节级兼容）。
- **网络 IO**：推荐 `SocketAsyncEventArgs`（少分配、长稳）；备选 `System.IO.Pipelines`。
- **包处理预算**：主线程每帧处理最多 N 个包（例如 200），避免洪峰卡顿。

---

## 2. 工程结构（建议）

### 2.1 解决方案目录

- `Client.MG.App/`
  - MonoGame `Game` 主循环、窗口、输入、场景切换、调试 overlay。
- `Client.MG.Render/`
  - `IRenderer2D`（SpriteBatch 封装、blend/shader 管理）、相机、批处理统计。
- `Client.MG.Content/`
  - `cpack` 读取、KTX2/atlas 元数据读取、缓存（LRU/引用计数）、异步加载（后续）。
- `Client.MG.Net/`
  - 连接管理、收发、解包线程、包队列、断线重连、流量统计。
- `Client.MG.Gameplay/`
  - 地图/对象/移动/聊天/战斗（后续）等“纯逻辑层”。
- `Client.MG.UI/`
  - 保 Mir 风格 UI 框架（控件树、输入、文本、布局、皮肤）。
- `Tools.ContentPipeline/`
  - 离线：旧资源 → atlas → KTX2 → pack（并生成 manifest）。

### 2.2 与现有 C# Client 模块对照（保证功能覆盖）

- 场景：`MirScenes/LoginScene.cs`、`SelectScene.cs`、`GameScene.cs` → `Scene` 系统。
- 对话框/UI：`MirScenes/Dialogs/*.cs`（36 个目录项）→ 新 UI 模块按功能拆分。
- 对象：`MirObjects/*Object.cs`、`SpellObject.cs`、`Effect.cs` → `EntityView` + `WorldState`。
- 寻路：`MirObjects/PathFinder.cs` → 新寻路模块（A*/JPS）。
- 网络：`MirNetwork/Network.cs` → 新网络模块。
- 图库：`MirGraphics/MLibrary.cs` → 新内容系统（atlas + KTX2 + pack）。

---

## 3. 运行时架构（线程模型与数据流）

### 3.1 主线程（MonoGame GameLoop）

- `Update()`：
  - 消费网络包（限额）→ 驱动 `WorldState` 更新。
  - 输入处理：鼠标点地 → 寻路 → 生成 direction 队列。
  - 本地预测/插值：玩家与其他对象的平滑移动。
  - UI 更新：聊天输入、窗口交互。
- `Draw()`：
  - 地图层 → 对象层 → 特效层 → UI 层 → Debug overlay。

### 3.2 网络线程

- Socket 接收 → ring buffer/byte buffer → 解包 → `Channel<Packet>`。
- Socket 发送：主线程把逻辑包写入 `send queue`，网络线程批量发送。

### 3.3 内容加载线程（建议后续做异步）

- pack 读取 + KTX2 解码可能较重：
  - MVP 阶段可同步/小资源同步；
  - 后续改为后台任务 + 主线程仅创建 GPU 资源。

---

## 4. MVP（登录 + 进图 + 走动 + 聊天）详细落地

### 4.1 里程碑 V0-V4（可交付切片）

- **V0：渲染骨架**
  - 开窗、60FPS、输入、Debug overlay（FPS/队列长度/耗时）。
- **V1：网络连通 + 聊天闭环**
  - 连接/收发/解包线程；聊天发送与展示；包处理预算生效。
- **V2：登录 + 进图（地图可先占位）**
  - 登录 UI；进入游戏场景；收到地图/位置后进入 world。
- **V3：点地寻路 + 方向步进走动同步**
  - 点击地面 → A* → direction 队列；按 tick 发送 `CWalk/CRun`；服务器修正。
- **V4：接入 KTX2 单包 + 基础地图/精灵渲染**
  - atlas.ktx2 + atlas.json + content.cpack；地图与人物精灵可见。

### 4.2 点地移动（客户端寻路 + 方向步进协议）

- 输入：鼠标点击屏幕坐标 → 转换地图格坐标 (tx, ty)。
- 寻路：基于阻挡网格 `WalkableGrid` 执行 A*（MVP 先 A*）。
- 输出：路径格序列 → 转成 `Vec<direction>`。
- 发送：
  - 每 `move_tick`（例如 100ms）取一个 direction：
    - 走：`CWalk { direction }`
    - 跑：`CRun { direction }`
- 修正：
  - 收到 `SUserLocation`/对象 walk/run 广播：
    - 偏差小：插值纠正
    - 偏差大：清空队列并重算
- 边界：
  - 点到不可走：BFS 寻找最近可走点
  - 最大路径长度限制（例如 200 步）
  - 连点覆盖旧路径

---

## 5. 全功能重写范围（覆盖现有 Client 所有模块）

> 下列功能按“系统域”拆分，并对应现有 C# 文件群，作为完整覆盖清单。

### 5.1 启动器/更新器/配置

- 启动流程：现有 `Program.cs` 中 patcher/launcher 逻辑 → 新启动器（可独立进程）。
- 配置：分辨率、音量、按键、语言、账号记录。
- 日志与崩溃报告：崩溃时记录版本、显卡、最后 N 条日志。

### 5.2 账号/登录/选角/创角

- 对应：`LoginScene.cs`、`SelectScene.cs`、`NewCharacterDialog.cs`。
- 功能：登录、服务器列表/地址、选角、创角、删除角色。

### 5.3 地图与世界呈现

- 地图加载：map 文件解析（客户端侧只需渲染与阻挡），与服务端 MapInfo 对齐。
- 多层：地表/装饰/遮挡层；相机跟随；小地图/大地图。
- 天气/粒子：先复刻现有效果，后期 GPU 优化。

### 5.4 对象系统（玩家/怪物/NPC/物品）

- 对应：`PlayerObject.cs`、`MonsterObject.cs`、`NPCObject.cs`、`ItemObject.cs`、`MapObject.cs`。
- 功能：
  - 动作状态机（站立/走/跑/攻击/受击/死亡）
  - 同步：服务端广播驱动，客户端插值/动画
  - 名字/血条/状态图标显示

### 5.5 战斗与技能（后续阶段）

- 对应：`SpellObject.cs`、`Effect.cs`、技能/动作相关逻辑。
- 功能：
  - 普攻/远程/技能释放
  - 目标选择与锁定
  - 伤害飘字、受击反馈
  - Buff 显示与计时

### 5.6 背包/装备/角色面板/快捷栏

- 对应：`InventoryDialog.cs`、`CharacterDialog.cs`、`MainDialogs.cs` 等。
- 功能：
  - 背包格、物品 tooltip、拖拽/拆分/堆叠
  - 装备栏、耐久显示
  - 快捷栏/技能栏

### 5.7 NPC/商店/合成/修理/寄售/游戏商城

- 对应：`NPCDialogs.cs`、`GameshopDialog.cs`、`TrustMerchantDialog.cs`。
- 功能：
  - NPC 对话与购买/出售
  - 合成/修理
  - 拍卖/寄售/商城

### 5.8 任务/邮件/社交/帮会/组队/排行

- 对应：`QuestDialogs.cs`、`MailDialogs.cs`、`FriendDialog.cs`、`GuildDialog.cs`、`GroupDialog.cs`、`RankingDialog.cs`。
- 功能：
  - 任务追踪与任务文本
  - 邮件收发
  - 好友/黑名单
  - 帮会/组队面板
  - 排行榜

### 5.9 聊天与输入系统

- 对应：`ChatOptionDialog.cs`、`ChatNoticeDialog.cs`、`MirTextBox.cs` 等。
- 功能：
  - 多频道、过滤、颜色
  - 链接物品/复制
  - 输入法（IME）与快捷键

### 5.10 音频

- 对应：`MirSounds/*`（现项目使用 NAudio）。
- 功能：
  - BGM/音效/环境音
  - 音量分组与淡入淡出
  - 避免频繁解码分配（缓存短音效）

---

## 6. 资源置换与内容管线（KTX2 单包）

### 6.1 离线管线输出

- `content.cpack`：单文件 pack。
- `atlas/*.ktx2`：按域分图集（ui/items/tiles/objects/actors/effects）。
- `atlas/*.json`：sprite 元数据（rect/offset/pivot/mask/tint 等）。
- `manifest.toml/json`：版本、hash、依赖、分组。

### 6.2 运行时加载策略

- 缓存：
  - atlas 级别缓存（引用计数 / LRU）
  - map 切换释放无关 atlas
- 解码阶段：
  - MVP：CPU 解码 RGBA 上传
  - 性能阶段：BasisU 转码到 GPU 压缩格式（平台相关）

### 6.3 体积与性能目标

- 目标 1：包体相对当前资源目录 **至少下降 30%**（视素材而定）。
- 目标 2：地图切换加载时间可量化并持续优化。

---

## 7. 性能与稳定性工程（贯穿全程）

### 7.1 指标与监控

- 帧时间：平均、P95、P99。
- 网络：收发字节、包队列长度、每帧处理包数量。
- 渲染：draw calls、纹理切换次数、atlas 命中率。

### 7.2 内存与分配

- 网络缓冲：ArrayPool/RingBuffer。
- 逻辑临时对象：对象池。
- 避免每帧大量 `new List/Vec`。

### 7.3 崩溃与日志

- 错误分级：info/warn/error。
- 崩溃报告：版本、平台、显卡、最近日志。

---

## 8. 分阶段路线（MVP → 全功能）

### 阶段 A（MVP 可玩）

- 登录 + 进图 + 点地走动 + 聊天
- KTX2 单包接入（可先 CPU 解码）

### 阶段 B（核心玩法补齐）

- 背包/装备/角色面板
- NPC 对话/商店/修理/合成
- 基础技能与战斗反馈

### 阶段 C（社交与系统）

- 任务/邮件/好友/组队/帮会/排行
- 商城/寄售/交易

### 阶段 D（性能与跨平台收口）

- KTX2 GPU 转码（Win: BCn / Mac: ASTC）
- 资源热更新/差分补丁（可选）
- Mac 构建与发布

---

## 9. 风险清单与应对

- **IME/文本输入**：尽早 PoC，避免后期返工。
- **KTX2 运行时方案**：MVP 先 CPU 解码，性能阶段再引入 native 转码。
- **点地移动与服务器校验**：必须处理服务器修正与路径重算，避免橡皮筋。
- **地图层级与遮挡一致性**：先复刻再优化，确保视觉一致。

---

## 10. 近期行动项（建议）

1) 先创建 `Client.MG.App` 空项目跑起来（V0）。
2) 在 RustServer 侧准备最小对接包清单（登录/StartGame/MapChanged/UserLocation/Chat/Walk/Run）。
3) 同步推进 `Tools.ContentPipeline`：先做 UI atlas（最容易验证 KTX2/pack）。

---

## 11. 协议与包清单（客户端实现顺序建议）

本节目的是把“重写范围”落到可实现的**包清单**，避免 UI/玩法做了但协议未覆盖。

### 11.1 MVP（登录+进图+走动+聊天）所需包

客户端→服务端（示例）

- **连接/鉴权**：`ClientVersion`、`Login`、`StartGame`、`KeepAlive`
- **移动**：`Turn`、`Walk(direction)`、`Run(direction)`
- **聊天**：`Chat(message, linked_items)`

服务端→客户端（示例）

- **进图**：`SMapChanged`（地图切换/进入地图）
- **位置**：`SUserLocation`（自己的坐标/方向更新）
- **对象移动广播**：`SObjectWalk`、`SObjectRun`、`SObjectTurn`
- **聊天广播**：`SChat`（或对应的 chat 包）

### 11.2 后续阶段包域（按系统归类）

- **背包/装备**：MoveItem/Store/TakeBack/Equip/Remove/Split/Use/Drop 等
- **NPC/商店**：Buy/Sell/Craft/Repair/Storage 等
- **战斗/技能**：Attack/RangeAttack/Magic/MagicKey/SpellToggle 等
- **社交/组队/帮会**：Group/Guild/Mentor/Friend 等
- **任务/邮件**：Quest/Mail 相关包

---

## 12. UI 重写与迁移清单（按现有 Dialog/Panel 分组）

目标是“保留 Mir 风格”，所以 UI 不建议一次性推翻；按功能域逐个迁移。

### 12.1 入口场景

- Login：`LoginScene` + `NoticeDialog`
- Select：`SelectScene` + `NewCharacterDialog`

### 12.2 核心面板（优先级最高）

- 主界面/快捷栏：`MainDialogs`
- 背包：`InventoryDialog`
- 角色：`CharacterDialog`
- 小地图/大地图：`CompassDialog`、`BigMapDialog`
- 聊天选项：`ChatOptionDialog`、`ChatNoticeDialog`

### 12.3 玩法系统面板（阶段性接入）

- 帮会：`GuildDialog`、`GuildTerritoryDialog`
- 组队：`GroupDialog`
- 任务：`QuestDialogs`
- 邮件：`MailDialogs`
- 好友/关系：`FriendDialog`、`RelationshipDialog`
- 排行：`RankingDialog`
- 交易：`TradeDialogs`
- 托管商人：`TrustMerchantDialog`
- 钓鱼：`FishingDialog`
- 坐骑：`MountDialog`
- 帮助：`HelpDialog`

---

## 13. 内容管线细化（旧资源 → KTX2 单包）

### 13.1 图集分组建议

为降低显存峰值与加载时间，图集必须按域拆分（而不是一个超级 atlas）。

- `ui/*`
- `items/*`
- `map/tiles/*`
- `map/objects/*`
- `actors/*`
- `effects/*`

### 13.2 渲染侧约束（保证批处理）

- 每个渲染 pass 尽量只绑定少量 atlas。
- 同层渲染按 atlas 分组，减少 `SpriteBatch` 的 state break。
- mask/tint（若存在）优先通过 shader 实现，避免二次绘制导致 draw call 翻倍。

---

## 14. 里程碑与时间表（建议）

以下为“可落地的工程节奏建议”，具体以团队人力调整。

### 14.1 MVP（4-6 周）

- 第 1 周：V0 + V1（渲染骨架 + 网络 + 聊天）
- 第 2 周：V2（登录 + 进图占位）
- 第 3-4 周：V3（点地寻路 + 方向步进移动 + 修正）
- 第 5-6 周：V4（KTX2 单包接入 + 基础地图/精灵可见）

### 14.2 全功能（分域并行推进）

- 阶段 B：背包/角色/主界面 + NPC 商店
- 阶段 C：任务/邮件/社交/帮会/组队/交易
- 阶段 D：性能与跨平台（KTX2 GPU 转码 + Mac）

---

## 15. 验收标准（建议写入 CI/QA 清单）

### 15.1 性能

- **帧时间**：P99 < 20ms（60FPS 目标下）
- **网络洪峰**：包队列增长时仍能保持 UI 响应（包处理预算生效）

### 15.2 稳定性

- 地图频繁切换不泄漏、不崩溃
- 长时间挂机运行（例如 4-8 小时）无明显内存增长

### 15.3 可观测性

- 错误/断线原因可在日志定位
- 能导出基础运行指标（帧率、队列长度、加载耗时）

---

## 16. 发布与更新策略（建议）

- 启动器/更新器建议独立进程：负责校验与下载 `content.cpack`。
- 资源按 manifest 版本管理：支持回滚到上一版资源包。
- 客户端程序与资源包分离更新：降低热更新风险。

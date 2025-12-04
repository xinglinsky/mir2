# Crystal 服务端 C++23 重写计划 v1

> 目标：在保证与原 Crystal C# 服务端逻辑与手感高度一致的前提下，使用现代 C++23 完成服务端全重写，并为未来多平台部署与长期维护打好基础。

---

## 一、背景与总体目标

- **现状**
  - 线上主力版本仍为 C# 服务端 + C# 客户端，WinForms / 同步逻辑为主。
  - C++23 重写以 **现有 C# 代码库为唯一行为基线**，不依赖 Rust 重写实现。
  - 希望有一份 C++ 技术栈实现，用于：
    - 与既有 C++ 生态（工具链、中间件）更好集成。
    - 试验基于 C++23 协程 / executors / async I/O 的实现路线。

- **C++23 重写的总体目标**
  - 使用 **C++23 + 标准库并结合少量第三方库**，在 Windows（后期可扩展到 Linux）上实现高性能游戏服务器。
  - **先 1:1 复刻行为，再逐步优化实现与架构**，保证玩家手感不变。
  - 与原 C# 版本在协议、数据结构和数值公式上保持完全兼容。

- **初步 Workspace / 模块规划（服务端）**
  - `crystal_cpp_foundation`：
    - 基础数据模型与共用类型（ID、枚举、坐标、属性等）。
    - 协议定义与编码/解码逻辑（C# `ServerPackets` / `ClientPackets` 的 C++ 复刻）。
  - `crystal_cpp_game`：
    - 世界/地图/怪物/战斗/AI/Buff 等核心游戏逻辑。
    - 对应 C# `Server/MirEnvir`、`Server/MirObjects`、大部分 `Server/Helpers`。
  - `crystal_cpp_infra`：
    - 基础设施：网络（基于 `asio`）、数据库访问、日志等。
    - 对应 C# `Server/MirNetwork`、`Server/MirDatabase` 及相关持久化逻辑。
  - `crystal_cpp_admin`：
    - GM 管理接口与 Web 后台 API 层，承载原 WinForms 管理工具的业务能力。
  - `crystal_cpp_server`（可执行）：
    - 服务器主进程，装配 `foundation` / `game` / `infra` / `admin` 并运行主循环。

---

## 二、总体原则

### 2.1 行为优先：先黑盒一致，再谈优化

- **与 C# 原服的行为保持一致**：
  - 协议字段含义与编码顺序不变。
  - Tick 顺序、怪物 AI 决策顺序、Buff 生效时机尽量对齐。
  - 同样场景下，怪物走位、攻击节奏、技能前后摇肉眼感受应一致。

- **禁止一开始就“过度聪明”**：
  - 不在初期阶段引入复杂 ECS 或过度并行调度。
  - 不在行为未锁定前修改数值公式、随机算法等。

### 2.2 代码质量与现代 C++23 特性

- **优先使用（新代码默认要求采用）**：
  - 强类型枚举 `enum class`，结构化绑定，`std::optional` / `std::variant`。
  - 智能指针与 RAII，避免裸指针生命周期管理。
  - `std::chrono` / `std::stop_token` / 现代线程 API。
  - **C++ Modules**：逐步以模块单元替代传统头文件，按功能拆分为 `crystal.net`、`crystal.core.world` 等模块，减少编译时间并降低 ODR 问题风险。
  - **协程 + async I/O**：在网络 I/O、计时器、异步数据库调用等场景优先采用 `co_await` + 调度器模型，减少线程数量与锁竞争，提高整体稳定性。
  - 统一错误处理策略：预期错误用 `expected` 风格（可自实现），非预期错误用异常或统一错误返回码。

- **使用约束**：
  - 协程主要用于 I/O 密集和定时任务，不在高频纯计算路径滥用，避免调度开销反而降低性能。

### 2.3 Modules / 协程的落地方向（硬性建议）

- **Modules**：
  - 协议与类型定义（ID、枚举、基础数据结构）抽到独立模块，供 `crystal_cpp_game`、`crystal_cpp_infra` 等共享，减少重复包含.
  - `crystal_cpp_game` 内部按子域拆分模块（例如 world/map/combat），控制编译单元大小并提高增量编译速度.
- **协程**：
  - `crystal_cpp_infra` 中的网络层：连接读写、心跳、超时控制统一以协程形式实现，避免大量回调与复杂状态机.
  - 定时任务与 Timer：Buff 过期、技能 CD 等可封装为协程友好的时间事件，配合统一调度器运行.

---

### 2.4 UI / 管理工具策略：不迁移 WinForms，改用 Web 后台

- 原 C# 工程中的 `Server.MirForms`（WinForms 管理工具 / GM 面板）**不做 UI 级别迁移**：
  - 不再在 C++23 工程中实现等价的桌面 UI.
  - 只复刻其中的业务逻辑与管理能力（如 GM 命令、监控、配置调整）。

- C++23 版本将：
  - 在服务端内提供一组 Web API（HTTP/REST 或 gRPC），暴露原有 GM / 管理能力.
  - 通过独立的 Web 前端（不在本计划范围内详细设计）实现可视化管理界面.
- 除服务端 UI 层外，其余所有逻辑层（世界、怪物、战斗、地图、掉落、账号/角色等）都按 C# 源码**逐文件/逐类 1:1 复刻**，以 C# 为权威行为定义.

---

## 三、阶段规划总览

- **阶段 0：协议与最小可运行骨架**
- **阶段 1：行为 1:1 复刻（单线程 / 轻多线程）**
- **阶段 2：内部轻量优化（数据结构、内存布局、Timer）**
- **阶段 3：中型优化（统一帧循环、多线程调度器、日志与监控）**
- **阶段 4：高级优化与特性扩展（可改变行为，需要公告）**

每一阶段完成后，进行一次回归测试与性能评估，再进入下一阶段.

---

## 四、阶段 0：协议与最小可运行骨架

**目标**：搭建起能与现有 C# 客户端通信的最小 C++ 服务器骨架，包括：协议编解码、连接管理、最简单的世界循环（可先 stub）。

### 4.1 协议子系统

- 以 C# 版 `ServerPackets` / `ClientPackets` 为协议真值，整理出独立的协议说明文档：
  - 确认包头结构、包长、指令号等一致.
  - 在 C++ 中定义所有请求/响应结构体，提供：
    - `encode()`：编码为字节流.
    - `decode()`：从字节流解析.
- 约束：
  - 不改变任何字段顺序与大小.
  - 增加的调试字段只能通过独立 GM/调试指令实现.

### 4.2 网络层

- 推荐基于 `asio`（可使用 standalone Asio 或 Boost.Asio）：
  - 实现连接接受/关闭、读取包长 + 包体、写队列等.
  - 每个连接维护一个 `Connection` 对象，内部持有：
    - 当前阶段（登录 / 选角 / 游戏内）。
    - 收包缓存、发包队列.

- 初期可采用：
  - 一个 I/O 线程 + 一个逻辑线程（或同线程）模型.
  - 逻辑线程周期性从队列中取出解码后的消息，调用 World API.

### 4.3 最小 World 骨架

- 提供最简单的几类结构：
  - `World`：持有地图列表、在线玩家列表.
  - `Player`：仅包含位置、等级、经验等基础字段.
  - `Map`：仅包含尺寸与简单地形信息.

- 支持的最小指令：
  - 登录 / 选角 / 进入游戏.
  - 简单移动（单步 / 简单寻路）。

---

## 五、阶段 1：行为 1:1 复刻

**目标**：在 C++ 服务端中完整实现现有 Crystal 行为（以 C# & Rust 实现为参考），优先保证功能与手感一致.

### 5.1 模块拆分

- `crystal_cpp_game` 内部推荐划分：
  - 玩家模块：属性、背包、装备、技能、Buff 列表等.
  - 怪物模块：AI 状态机、仇恨列表、掉落规则.
  - 战斗模块：伤害计算、命中/闪避/暴击、Buff 施加与移除.
  - 地图与传送模块：地图加载、传送点、刷怪点配置.

### 5.2 与 C# 行为对齐

- 对照 C# 版本：
  - 怪物刷怪频率与路径.
  - 技能伤害公式与冷却时间.
  - Buff 行为（持续时间、叠加规则）。

- 建议引入简单的 **Record/Replay 工具**：
  - 记录一段玩家在旧服上的操作序列.
  - 在 C++ 服上重放，比较关键结果（血量、怪物死亡时间、奖励）。

---

## 六、阶段 2：内部轻量优化

**目标**：在不改变对外行为的前提下，通过更合理的数据结构与内存布局优化性能.

### 6.1 数据结构与内存布局

- 使用 `std::vector` / `std::deque` 替代链表式结构.
- 对频繁访问的对象（如玩家、怪物）采用：
  - 索引池（`std::vector` + 空位列表）代替散乱分配.
  - 紧凑结构体、避免过多虚函数与间接跳转.

### 6.2 Timer 与调度

- 将 Buff 到期、技能冷却、怪物 AI 间隔统一纳入一个时间轮或优先队列.
- 保证同一 Tick 内事件顺序与旧服一致（如按插入顺序处理）。

---

## 七、阶段 3：统一帧循环与多线程调度

**目标**：引入统一的帧循环（Frame Loop），并根据需要引入多线程调度器，以提升多核利用率.

### 7.1 统一帧循环

- 定义 `FrameState { uint64_t frame_no; std::chrono::milliseconds dt; }`.
- 世界更新接口统一为 `World::update(const FrameState&)`.
- 所有子系统（AI / Buff / 掉落 / 地图事件）在此接口下有固定调用顺序.

### 7.2 多线程调度（可选，逐步启用）

- 以“地图”为分片：
  - 每个 Worker 线程负责若干地图的全部逻辑.
  - 跨地图事件通过事件队列在帧尾统一合并.

- 所有写操作视为发生在帧结束时，避免竞态和“有人看到新状态，有人还在旧状态”的问题.

---

## 八、阶段 4：高级优化与新特性（可改变行为）

**注意：本阶段内容可能改变玩家体验，应视为新版本特性，需要公告与灰度发布.**

- AOI 重构：
  - 从简单九宫格升级为可配置半径、分层 AOI 模型.
- 寻路重构：
  - 支持更精细的 navmesh / 栅格寻路，提高怪物走位质量.
- 战斗系统扩展：
  - 更复杂的抗性 / 元素体系.
  - 更丰富的 Buff / Debuff 组合.

---

## 九、与原 C# 代码库的对应关系（按文件粒度复刻）

- 以当前 C# 服务器仓库为**唯一权威实现**，C++23 重写按“文件 / 类”为粒度逐步迁移：
  - 为 `Server/MirEnvir/Envir.cs`、`Server/MirObjects/PlayerObject.cs` 等核心文件，各自建立对应的 C++ 模块或 `.cpp` 文件.
  - 尽量保持同名或相近命名的类型与方法，方便对照与 diff.
- 迁移策略建议：
  - 先从与 WinForms/GUI 耦合最少的核心逻辑开始（世界、地图、怪物、战斗、掉落等）。
  - 每迁移完一组 C# 文件，就在 C++ 侧补齐对应单元测试或最小集成测试，确保行为与 C# 原实现一致.
- C++23 工程只依赖 C# 仓库作为“行为文档”，**不再依赖 Rust 重写工程**；未来如需与其他语言实现共享数据/规则，可再单独设计语言无关的模型描述层（如 JSON/YAML/proto schema）。

### 9.1 首批必须照抄的 C# 文件与目标 C++ 模块

> 这一批文件是 M0/M1 阶段优先迁移对象，迁移时要求以 C# 为行为真值，逐函数比对.

| C# 源文件 | 建议 C++ 模块 / 命名空间 | 角色说明 |
| --- | --- | --- |
| `Server/MirEnvir/Envir.cs` | `crystal_cpp_game::world::envir` | 全局世界管理、Tick 驱动、刷怪与环境事件调度. |
| `Server/MirEnvir/Map.cs` | `crystal_cpp_game::world::map` | 地图数据结构、格子状态、刷怪点与传送点逻辑. |
| `Server/MirEnvir/Dragon.cs` | `crystal_cpp_game::world::boss_dragon` 或 `::world::special::dragon` | 特殊 Boss/事件型地图逻辑. |
| `Server/MirObjects/PlayerObject.cs` | `crystal_cpp_game::objects::player` | 玩家实体：属性、经验/等级、背包、技能、Buff、组队等. |
| `Server/MirObjects/MonsterObject.cs` | `crystal_cpp_game::objects::monster` | 怪物实体：AI 状态机、仇恨、移动与掉落入口. |
| `Server/MirObjects/HeroObject.cs` | `crystal_cpp_game::objects::hero` 或 `::objects::pet` | 英雄/宠物实体：与玩家绑定的战斗单位. |
| `Server/MirDatabase/AccountInfo.cs` | `crystal_cpp_infra::db::account_info` | 账号信息持久化结构. |
| `Server/MirDatabase/CharacterInfo.cs` | `crystal_cpp_infra::db::character_info` | 角色基础信息（职业、等级、经验等）持久化结构. |
| `Server/MirDatabase/AuctionInfo.cs` | `crystal_cpp_infra::db::auction_info` | 拍卖行条目与相关配置. |
| `Server/MirDatabase/BuffInfo.cs` | `crystal_cpp_infra::db::buff_info` | Buff 定义及其持久化字段. |
| `Server/MirNetwork/MirConnection.cs` | `crystal_cpp_infra::net::game_connection` | 主游戏连接：登录后到游戏内所有协议的承载. |
| `Server/MirNetwork/MirStatusConnection.cs` | `crystal_cpp_infra::net::status_connection` | 状态/监控连接（在线人数、心跳等）。 |
| `Server/Helpers/ChatSystem.cs` | `crystal_cpp_game::systems::chat` 或 `::world::chat` | 公共/私聊/行会聊天等消息路由与过滤. |

后续可按同一模式扩展列表，例如：

- `Server/MirObjects/<Name>Object.cs` → `crystal_cpp_game::objects::<name>`.
- `Server/MirDatabase/<Name>Info.cs` → `crystal_cpp_infra::db::<name>_info`.
- `Server/Helpers/<Name>.cs` → 与其职责对应的 `crystal_cpp_game::systems::<name>` 或 `crystal_cpp_admin::helpers::<name>`.

---

## 十、里程碑与验收

- **M0：协议与最小骨架完成**
  - C++23 服务器可接受连接、完成登录/选角/进入游戏/简单移动。

- **M1：核心战斗与怪物 AI 复刻完成**
  - 常见地图与副本在 C++ 服上可跑通，体验接近原服。

- **M2：内部轻量优化完成**
  - 在测试环境中，CPU 与内存占用优于 C# 原服，接近或优于 Rust 版。

- **M3：统一帧循环与（可选）多线程调度完成**
  - 在实际压测中，多核利用率明显提升，卡顿尖刺减少。

- **M4：高级优化与新特性灰度上线**
  - AOI / 寻路 / 战斗扩展等新特性通过小范围灰度验证后推广。

> 当前进度简要：
> - 已在 `CppServer/` 下完成 M0 基础骨架搭建（CMake 工程、crystal_cpp_game/infra/server 目标）。
> - 已按 C# Envir/Map/Player 与 Rust ExpList 逻辑，在 `crystal_cpp_game::world` 中搭建世界/地图/玩家与经验表的骨架，并实现经验累积与基础升级循环。
>
> 本文档为 Crystal 服务端 C++23 重写的初版路线图，后续可根据实际进度与新需求持续演进更新。

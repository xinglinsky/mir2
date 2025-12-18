# Rust 客户端目录结构设计（对照 C# Client）

本文目标：基于现有 C# 客户端目录/类的职责边界，给出 Rust（Bevy）重写时推荐的 **workspace + crate + module（精确到文件名）** 结构。

设计原则：

- **结构清晰**：按职责拆 crate；crate 内按 feature 拆 module。
- **文件大小合理**：避免“一个文件上千行”，对标 C# 中超大文件（如 `GameScene.cs`、`PlayerObject.cs`、`MonsterObject.cs`、`MainDialogs.cs`）拆分为多个子模块。
- **可渐进迁移**：允许先实现最小闭环（login → select → enter world），后续逐步补齐游戏 UI/对象同步/地图渲染。
- **可测试**：协议、资源解码、内容加载尽量独立 crate，便于单元测试/Golden tests。

---

## 1. C# 客户端结构速览（输入）

C# `Client/` 核心目录：

- **入口/配置**：`Program.cs`, `Settings.cs`, `KeyBindSettings.cs`
- **WinForms 主窗体/启动器**：`Forms/CMain*.cs`, `Forms/AMain*.cs`, `Forms/Config*.cs`
- **网络**：`MirNetwork/Network.cs`
- **渲染/资源**：`MirGraphics/DXManager.cs`, `MirGraphics/MLibrary.cs`, `MirGraphics/ParticleEngine.cs`
- **UI 控件体系**：`MirControls/*.cs`（`MirControl/MirButton/MirTextBox/...`）
- **场景**：`MirScenes/LoginScene.cs`, `MirScenes/SelectScene.cs`, `MirScenes/GameScene.cs`, `MirScenes/Dialogs/*.cs`
- **游戏对象**：`MirObjects/*Object.cs`, `Frames.cs`, `MapObject.cs`, `PathFinder.cs`...
- **声音**：`MirSounds/SoundManager.cs`, `MirSounds/Libraries/*.cs`

对应关系（粗粒度）：

- C# `MirScenes` → Rust `scene_*`（login/select/game）
- C# `MirControls` → Rust `ui_*`（widgets/components/theme/input）
- C# `MirNetwork` → Rust `crystal-client-net`（已存在）
- C# `MirGraphics` + `MLibrary` → Rust `crystal-lib`/`asset_*`/`render_*`（部分已存在）

---

## 2. Rust workspace 总体布局（建议）

位置：`RustServer/`（你当前 workspace 已在这里）。

建议保留现有 crates，并新增/重构以下客户端相关 crates：

```text
RustServer/
  Cargo.toml

  crystal-client-bin/               # 最小启动入口（二进制）；只做 bootstrap + CLI
  crystal-client-app/               # 新：Bevy App 组装（plugins/feature flags/资源初始化）
  crystal-client-config/            # 已有：配置读取/写入
  crystal-client-net/               # 已有：Tokio TCP + 收发队列 + 事件
  crystal-client-proto/             # （可选）新：仅客户端侧 packet encode/decode 辅助（如果不想都放 shared-proto）
  crystal-lib/                      # 已有：.Lib 解码（MLibrary/MImage 等）

  crystal-client-asset/             # 新：资产定位/加载（Lib 索引、音频、字体、地图资源索引）
  crystal-client-render/            # 新：渲染层（2D sprite、tilemap、shader、batch）
  crystal-client-ui/                # 新：UI 框架层（widgets/theme/layout/input focus）
  crystal-client-scene/             # 新：场景状态机（login/select/game + overlay dialogs）
  crystal-client-world/             # 新：客户端世界状态（玩家/对象/地图/时间/效果）
  crystal-client-audio/             # 新：音频播放（BGM/SFX/缓存/通道管理）

  crystal-content-pack/             # 已有/或未来：内容工具（可选）
  ...
```

说明：

- `crystal-client-bin` **必须保持很薄**：解析 CLI、加载 config、调用 `crystal-client-app::run()`。
- “大逻辑”都下沉到 `*-app/scene/ui/world/render/asset` 等库 crate。

---

## 3. 各 crate 详细目录结构（精确到文件名）

### 3.1 `crystal-client-bin`（binary）
对标 C# `Program.cs`（入口+patcher/launch decisions）。

```text
crystal-client-bin/
  Cargo.toml
  src/
    main.rs               # 薄入口：声明模块 + 委托到 main_old（过渡期）
    main_old.rs           # 过渡期：mode dispatcher（解析 CLI + 构造 config + 路由到 modes::*）
    cli.rs                # clap 定义（login-ui、lib-test、server/account/password 等）
    app_config.rs         # RuntimeConfig/LibTestConfig/LoginUiConfig
    shared/
      mod.rs
      lib_image.rs        # .Lib -> Bevy Image helpers
    modes/
      mod.rs
      lib_test.rs
      login_ui/
        mod.rs
        state.rs
        ui.rs
        net.rs
      runtime_chat/
        mod.rs
        state.rs
        ui.rs
        net.rs
        router.rs
```

Planned（下一步拆分目标，仅文件名与职责边界；未落地前不保证 API 稳定）：

```text
crystal-client-bin/
  src/
    shared/
      net_router.rs       # 统一 pump net events + packet dispatch（login_ui/runtime_chat 共享）
      logging.rs          # key log/unhandled log（文件落盘 + UI 抑制策略）
    # （后续）login_ui/runtime_chat 将逐步复用 shared/net_router + shared/logging
```

文件规模：

- `main.rs` 控制在 ~150 行以内。

---

### 3.2 `crystal-client-app`（Bevy App 组装）
对标 C# `CMain`（游戏主循环、初始化顺序、全局资源）。

```text
crystal-client-app/
  Cargo.toml
  src/
    lib.rs                # pub fn run(cfg: RuntimeConfig)
    app_builder.rs        # build_app(): App
    plugins.rs            # ClientPlugins 列表：net/scene/ui/render/audio
    resources.rs          # 全局资源：RuntimeConfig、Time、Diagnostics 等
    startup.rs            # startup systems：load config/asset prewarm
    shutdown.rs           # graceful exit、日志 flush
```

---

### 3.3 `crystal-client-config`（已存在，建议细化）
对标 C# `Settings.cs` + `KeyBindSettings.cs`。

```text
crystal-client-config/
  Cargo.toml
  src/
    lib.rs
    model.rs              # ClientConfig struct（graphics/network/audio/game/ui）
    io.rs                 # load_or_default/save
    paths.rs              # DataPath/MapPath/SoundPath 等统一 Path 解析
    migrate.rs            # 版本迁移（可选）
    keybinds.rs           # 键位映射（后续）
```

---

### 3.4 `crystal-client-net`（已存在，建议细化）
对标 C# `MirNetwork/Network.cs`（connect/retry/recv/send queues）。

```text
crystal-client-net/
  Cargo.toml
  src/
    lib.rs
    client.rs             # NetClient：connect/send/recv
    codec/
      mod.rs
      raw_packet.rs       # RawPacket framing
    events.rs             # NetEvent enum
    reconnect.rs          # 重连策略（最大次数/间隔/backoff）
    metrics.rs            # bytes sent/recv，诊断信息
```

---

### 3.5 `crystal-lib`（已存在：.Lib 解码）
对标 C# `MirGraphics/MLibrary.cs` + `MImage`。

```text
crystal-lib/
  Cargo.toml
  src/
    lib.rs
    error.rs              # LibError
    reader.rs             # 读取 header/indexList/frameSet
    image.rs              # 解码单张图片（gzip BGRA→RGBA）
    mask.rs               # mask layer（如有）
    tests.rs              # lib 解码 golden tests（可选）
```

---

### 3.6 `crystal-client-asset`（新：资产加载/索引）
对标 C# `Libraries` 静态初始化（`ChrSel/Prguse/Title/...`，以及 MapLibs/Items/Monsters 等）。

```text
crystal-client-asset/
  Cargo.toml
  src/
    lib.rs

    paths.rs              # 根据 config 解析 Data/Map/Sound 路径

    libset/
      mod.rs
      lib_id.rs           # enum LibId { ChrSel, Prguse, Title, ... }
      lib_manifest.rs     # 每个 Lib 的文件名/用途/索引范围（文档化）
      lib_store.rs        # 缓存：LibId -> LibFile/Bevy Image handles

    sprites/
      mod.rs
      atlas.rs            # （可选）将常用 index 组装成 atlas
      handle_cache.rs

    fonts/
      mod.rs
      loader.rs

    maps/
      mod.rs
      map_libs.rs         # MapLibs 0..400 的路径规则
      map_manifest.rs

    sound/
      mod.rs
      sound_manifest.rs
```

---

### 3.7 `crystal-client-render`（新：渲染/图形）
对标 C# `DXManager`、`ParticleEngine`、地图渲染与特效。

```text
crystal-client-render/
  Cargo.toml
  src/
    lib.rs
    plugin.rs             # RenderPlugin

    camera.rs
    scaling.rs            # 分辨率/像素对齐（1024x768 基准）
    materials.rs          # shader/material 管理

    sprite/
      mod.rs
      draw.rs             # 通用 sprite draw helpers
      animation.rs        # frame animation（ChrSel 背景等）

    tilemap/
      mod.rs
      map_view.rs         # 当前地图可视区域
      layers.rs           # ground/object/light layers
      culling.rs

    particles/
      mod.rs
      particle_system.rs
```

---

### 3.8 `crystal-client-ui`（新：UI 框架与 widgets）
对标 C# `MirControls/*`（控件体系 + 输入焦点 + 皮肤/状态）。

```text
crystal-client-ui/
  Cargo.toml
  src/
    lib.rs
    plugin.rs

    theme/
      mod.rs
      colors.rs
      metrics.rs          # padding/字号/控件默认尺寸
      mir2_skin.rs        # 把 Title/Prguse 的按钮三态索引统一管理

    input/
      mod.rs
      focus.rs            # focus 管理（类似 WinForms ActiveControl）
      text_edit.rs         # 文本输入/过滤/光标/selection

    widgets/
      mod.rs
      image.rs            # Image widget（Lib index → UiImage）
      button.rs           # 三态按钮（base/hover/pressed/disabled）
      textbox.rs          # 黑底 textbox + border + caret
      label.rs
      window.rs           # 可拖动/可关闭窗口（对标 MirDialog）
      list.rs

    layout/
      mod.rs
      anchors.rs          # C# Point/Size 到 Bevy absolute layout helper

    debug/
      mod.rs
      overlay.rs          # FPS/调试信息
```

说明：

- `widgets/button.rs` 统一实现 C# `MirButton` 的状态机：hover/pressed/disabled。
- `widgets/textbox.rs` 实现 C# `MirTextBox` 的行为（黑底、边框红绿、caret）。

---

### 3.9 `crystal-client-scene`（新：场景状态机）
对标 C# `MirScenes/*`。

```text
crystal-client-scene/
  Cargo.toml
  src/
    lib.rs
    plugin.rs             # ScenePlugin

    state.rs              # enum SceneState { Launcher, Login, Select, Game }
    transitions.rs        # 切换逻辑（on_enter/on_exit）

    login/
      mod.rs
      scene.rs            # login scene root
      ui.rs               # login dialog 布局（使用 crystal-client-ui widgets）
      logic.rs            # 输入校验、触发 net login

    select/
      mod.rs
      scene.rs
      ui.rs
      logic.rs

    game/
      mod.rs
      scene.rs
      hud.rs              # 只放 HUD 框架，具体对话框在 dialogs/
      packet_router.rs    # game scene 内处理 server packets（细分到 world）

    dialogs/
      mod.rs
      common.rs           # 通用 dialog 基类/traits
      inventory.rs
      character.rs
      chat_options.rs
      guild.rs
      npc.rs
      main.rs             # 对标 C# MainDialogs.cs（但这里再拆文件）
```

文件拆分建议（对应 C# 超大 dialogs）：

- C# `MainDialogs.cs`（188KB） → `dialogs/main/{inventory,skills,chat,mini_map,...}.rs` 多文件。
- C# `NPCDialogs.cs`（95KB） → `dialogs/npc/{talk,quests,shop,...}.rs`。

---

### 3.10 `crystal-client-world`（新：客户端世界状态与对象同步）
对标 C# `MirObjects/*` + `GameScene.cs` 中的大量“对象/地图/战斗状态”。

```text
crystal-client-world/
  Cargo.toml
  src/
    lib.rs
    plugin.rs             # WorldPlugin

    state/
      mod.rs
      client_world.rs     # WorldState：地图信息、玩家、对象表、时间
      flags.rs

    map/
      mod.rs
      map_info.rs         # MapInformation/MapChanged
      tile.rs
      pathfinding.rs      # 对标 PathFinder.cs（可后置）

    objects/
      mod.rs
      object_id.rs
      components.rs       # Position/Direction/AnimationState

      player/
        mod.rs
        model.rs
        update.rs         # ObjectPlayer/UserInformation 更新

      monster/
        mod.rs
        model.rs
        update.rs

      npc/
        mod.rs
        model.rs
        update.rs

      item/
        mod.rs
        model.rs
        update.rs

    effects/
      mod.rs
      buffs.rs
      spells.rs
      damage.rs

    packets/
      mod.rs
      router.rs           # 根据 packet id 分发到各 handler
      handlers/
        mod.rs
        login.rs
        select.rs
        map.rs
        objects.rs
        chat.rs
```

拆分目标：

- 将 `GameScene` 中的“处理包 + 更新对象 + 渲染触发”拆成：
  - `crystal-client-net` 负责接收事件
  - `crystal-client-world` 负责更新状态
  - `crystal-client-render` 负责把状态映射到可视实体

---

### 3.11 `crystal-client-audio`（新：音频）
对标 C# `MirSounds/SoundManager.cs` + `MirSounds/Libraries/*`。

```text
crystal-client-audio/
  Cargo.toml
  src/
    lib.rs
    plugin.rs

    manager.rs            # SoundManager：音量/通道/缓存
    library/
      mod.rs
      cached_sound.rs     # 对标 CachedSound
      oneshot.rs          # OneShotProvider
      looping.rs          # LoopProvider

    events.rs             # PlaySfx/PlayBgm/Stop 等事件
```

---

## 4. 文件大小控制建议（硬性约束）

- **单个 `.rs` 文件**建议控制在：
  - **常规模块**：200~500 行
  - **复杂模块**：500~800 行（尽量再拆）
  - 超过 800 行必须拆分（尤其是 `packet_router`、`dialogs/main`、`objects/*`）。

拆分策略：

- **按“数据模型 / 处理逻辑 / 渲染绑定”分离**：
  - `model.rs`：struct/enum
  - `update.rs`：状态机更新、包处理
  - `view.rs`/`render.rs`：把状态映射到实体
- **按协议包域拆 handler**：login/select/map/objects/chat。

---

## 5. 与当前 Rust workspace 的落地方式（最小改动）

你当前已有：

- `crystal-client-bin`
- `crystal-client-config`
- `crystal-client-net`
- `crystal-lib`

建议落地顺序：

1. 新建 `crystal-client-app`：把现在 `crystal-client-bin/src/main.rs` 的大部分 bevy systems 下沉。
2. 新建 `crystal-client-ui`：把 login-ui demo 重构为 widgets + theme。
3. 新建 `crystal-client-scene`：用 `SceneState` 把 login/select/game 串起来。
4. 新建 `crystal-client-world`：承接 V2-2/V2-3（进图、对象同步）。

---

## 6. 附：C# → Rust 责任映射表（便于对照）

- **Program.cs** → `crystal-client-bin/src/main.rs` + `crystal-client-app/src/lib.rs`
- **Settings.cs** → `crystal-client-config/src/{model,io,paths}.rs`
- **MirNetwork/Network.cs** → `crystal-client-net/src/{client,events,reconnect}.rs`
- **MirGraphics/MLibrary.cs** → `crystal-lib/src/{reader,image,mask}.rs` + `crystal-client-asset`
- **MirGraphics/DXManager.cs** → `crystal-client-render`（camera/material/tilemap/particles）
- **MirControls/** → `crystal-client-ui/widgets/*`
- **MirScenes/LoginScene.cs** → `crystal-client-scene/login/*`
- **MirScenes/SelectScene.cs** → `crystal-client-scene/select/*`
- **MirScenes/GameScene.cs** → `crystal-client-scene/game/*` + `crystal-client-world/*`（拆！）
- **MirScenes/Dialogs/*.cs** → `crystal-client-scene/dialogs/*`（按业务拆文件）
- **MirObjects/*.cs** → `crystal-client-world/objects/*`
- **MirSounds/** → `crystal-client-audio/*`

---

## 7. 需要你确认的两个关键点（决定结构细节）

1. **你希望 Rust 客户端最终是否完全替代 C#（包括 patcher/launcher）？**
   - 若是：保留 `crystal-launcher-bin` 并扩展；`SceneState` 里加入 `Launcher`。
   - 若否：Launcher 只做最小启动；重点放 login/game。

2. **地图渲染方案**：
   - **方案 A（短期）**：先用 sprite/简单 tilemap + 仅地表层。
   - **方案 B（长期）**：定制 tile batching + 多层（地面/对象/光照/特效）。

确认后我可以把文档里的 `render/tilemap` 与 `asset/maps` 结构进一步精确到更多文件。

# Rust 服务端 GM 指令与运维工具使用手册

> 说明：本手册面向日常运维 / 调试，整理 **Rust 登录/游戏服务器中的 GM 聊天命令**，以及与数据库汉化相关的 **工具使用流程**。可配合 `docs/数据库汉化方案.md` 一起阅读。

---

## 一、Rust 服务端 GM / 管理命令

### 1.1 命令生效范围

- **输入位置**：游戏内普通聊天框（与 C# 版一致）。
- **处理流程**：
  - 文本进入 `LoginConnection::handle_chat`；
  - 先检查长度（>255 字符直接断开连接，视为恶意包）；
  - 然后调用 `handle_gm_chat(trimmed, out)` 尝试处理 GM 命令；
  - 若未处理，再处理一些特殊 NPC 触发命令（如 `@ADDSTORAGE`）。

### 1.2 GM 模式开关：`/gm`

- **命令**：
  - `/gm` 或 `/gm on`：开启 GM 模式；
  - `/gm off`：关闭 GM 模式；
- **效果**：
  - 设置连接上的 `is_gm` 标志；
  - 后续标记为 “GM only command.” 的命令只有在 `is_gm == true` 时才允许执行；
- **提示**：
  - 开启：`GM mode enabled.`
  - 关闭：`GM mode disabled.`

### 1.3 扩展仓库租赁：`/addstorage` 与 `@ADDSTORAGE`

- **命令**：
  - 聊天输入：`/addstorage`
  - 或由客户端界面触发：`@ADDSTORAGE`
- **权限**：不要求 GM，只要玩家在游戏中即可。
- **行为**：
  - 检查当前角色是否已登录并加载了角色属性；
  - 从账号级仓库（`AccountStorage`）加载或初始化数据；
  - 如果首次购买，将仓库存储格从 80 扩展到 160（mir 端经典扩展仓库逻辑）；
  - 扣除 **1,000,000 Gold**，并延长 10 天租期：
    - 若旧租期仍有效，则在原到期时间基础上 +10 天；
    - 否则从当前时间起 +10 天；
  - 保存角色属性和账号仓库，并下发：
    - `SLoseGold`（扣金币表现）；
    - `SResizeStorage`（扩展仓库大小和到期时间）。
- **错误提示示例**：
  - `Not in game.`
  - `Character stats not loaded.`
  - `Not enough gold for expanded storage.`
  - `Failed to load account storage.` / `Failed to save account storage.` 等。
- **@ADDSTORAGE 映射**：
  - `chat.rs` 中：当收到 `@ADDSTORAGE` 时，会内部调用 `handle_gm_chat("/addstorage", out)`，重用同一套逻辑。

### 1.4 帮会经验：`/guildexp`

- **命令**：
  - `/guildexp <amount>`
- **权限**：GM 仅限（`is_gm == false` 时返回 `GM only command.`）。
- **行为**：
  - 从 `player_summaries` 中获取当前玩家所在的公会名称；
  - 若玩家未加入公会，则提示：`You are not in a guild.`；
  - 调用 `world.guild_gain_exp(&guild_name, base_amount)` 增加公会经验；
  - 保存公会数据，并向公会成员广播经验增加：
    - 未升级：`Guild X gained Y experience.`
    - 升级：`Guild X gained Y experience and leveled up to level Z.`

### 1.5 创建帮会：`/createguild`

- **命令**：
  - `/createguild <GuildName>`
- **权限**：对普通玩家开放，但有条件限制。
- **行为 / 限制**：
  - 名称长度：3–20 字符，否则：
    - `Guild name must be between 3 and 20 characters.`
  - 不能包含 `\` 字符，否则：
    - `Guild name contains invalid characters.`
  - 玩家不能已经在帮会中：
    - `You are already part of a guild.`
  - 需要达到 `GuildSettings.ini` 中配置的最低等级：
    - `Guilds.MinimumLevel`（在 Rust 中通过 `guild_settings().required_level` 读取）；
    - 不满足时提示：`Your level is not high enough to create a guild, required: <Level>`。
  - 若公会名已存在：`Guild <name> already exists.`
- **成功效果**：
  - 在 world 中创建公会并持久化；
  - 将当前角色加入该公会并设为 `Leader`；
  - 下发：
    - `SObjectGuildNameChanged`：刷新场景中该角色头顶公会名；
    - `SGuildStatus`：给客户端公会界面使用；
  - 系统提示：`Successfully created guild <name>`。

> 备注：客户端 “创建帮会” 界面通常会先通过包 `CGuildNameReturn` 把名字提交给服务器，最终同样走到 `handle_create_guild_command`。

### 1.6 角色金币调试：`/showmemoney`

- **命令**：
  - `/showmemoney <delta>`
- **权限**：GM 仅限。
- **行为**：
  - 将 `<delta>` 解析为 `i64`，只处理 `delta > 0` 的情况（非正数等于无操作）；
  - 在当前角色属性上增加 `delta` 金币并保存；
  - 下发 `SGainedGold` 包给客户端，表现为获得金币。

> 注意：没有专门的使用提示，要自己保证只在测试环境使用，避免破坏经济平衡。

### 1.7 调整等级：`/level`

- **命令**：
  - `/level <level>`
- **权限**：GM 仅限。
- **行为 / 限制**：
  - 若未填写参数：`Usage: /level <level>`；
  - 参数必须能解析为 `u16` 且 `>= 1`；
  - 为防止溢出，若等于 `u16::MAX` 会被钳制为 `u16::MAX - 1`；
  - 经验值强制设为 0；
  - 调用 `world.set_player_level_and_experience` 更新世界中玩家等级与经验；
  - 重新从 world 拉取该角色的 **最大 HP/MP** 并写回当前属性；
  - 保存角色属性和魔法，并：
    - 发送 `SHealthChanged`（HP/MP 同步）；
    - 发送 `SLevelChanged`（等级 + 当前经验 + 当前等级的 MaxExperience）；
    - 广播 `SObjectLeveled` 给周围玩家（头顶升级特效）；
  - 同时更新 `characters` 列表中该角色的显示等级，保证角色选择界面同步。

### 1.8 学习技能：`/skill`

- **命令**：
  - `/skill`：为当前职业学习 **所有可学技能**；
  - `/skill <ID>`：按技能 ID 学习单个技能；
  - `/skill <Name>`：按 `MagicInfo.Name`（忽略大小写）查找并学习单个技能。
- **权限**：GM 仅限。
- **行为**：
  - 根据当前角色职业（战士/法师/道士/刺客/弓手）计算职业 ID；
  - 无参数：
    - 从 `world_db.magic_infos()` 读取全部技能列表；
    - 过滤：`spell != 0 && class_owns_spell(class_id, spell)`；
    - 为每个技能调用 `world.learn_magic_for_player`；
    - 对新学会的技能发送新增技能包，并在最后提示：`Learned N skill(s).`；
    - 若一个也没新增则提示：`No new skills learned.`
  - 有参数：
    - 先尝试解析为 `u8` 技能 ID，不行则按名称在 `magic_infos()` 中查找；
    - 若找不到：`Unknown skill.`；
    - 若技能不属于当前职业：`Skill does not belong to your class.`；
    - 若已经学会：`Skill already learned.`；
    - 成功学习后保存角色魔法列表，并提示：`Learned 1 skill(s).`。

### 1.9 杀死最近怪物：`/kill`

- **命令**：
  - `/kill`
- **权限**：GM 仅限。
- **行为**：
  - 调用 `world.kill_nearest_monster(current_map_index, current_x, current_y)`；
  - 若返回最近怪物 ID 及其经验：
    - 从 `known_monsters` 集合中移除该怪物；
    - 向客户端发送 `SObjectRemove` 包（怪物从场景中消失）；
    - 若经验 > 0，则向 world 投递 `WorldEvent::GainExperience` 事件，正常触发经验与升级逻辑。

---

## 二、数据库 / 语言相关工具

目前与数据库汉化、语言文件生成相关的主要有两个工具：

- `Tools.DatabaseExporter`：从 C# 服务器数据库导出英文名称到 CSV；
- `Tools.LanguageGenerator`：
  - 默认模式：从 `Shared/Languages.csv` 生成 **客户端/服务端 Language.ini**；
  - `merge` 模式：将 `docs/translations/*.csv` 中的翻译合并回 `Shared/Languages.csv`。

### 2.1 Tools.DatabaseExporter

#### 2.1.1 位置

- 工程路径：`Tools.DatabaseExporter/`
- 可执行文件（示例）：
  - `Tools.DatabaseExporter/bin/Debug/net8.0/Tools.DatabaseExporter.exe`

#### 2.1.2 启动参数

- **命令格式**：

  ```powershell
  Tools.DatabaseExporter.exe [<Configuration>]
  ```

- `<Configuration>`（可选）：
  - 指定服务器构建目录名，例如：`Debug` / `Release`；
  - 省略时默认 `Debug`。

- 从仓库根目录运行示例：

  ```powershell
  # 使用 Debug 配置（默认）
  .\Tools.DatabaseExporter\bin\Debug\net8.0\Tools.DatabaseExporter.exe

  # 显式指定 Release 配置
  .\Tools.DatabaseExporter\bin\Debug\net8.0\Tools.DatabaseExporter.exe Release
  ```

#### 2.1.3 服务器目录定位逻辑

程序会从解决方案根目录（mir2）出发，依次尝试以下目录作为服务器工作目录：

1. `Build/Server/<Configuration>`
2. `Build/Server/Debug`
3. `Build/Server/Release`

要求候选目录中同时存在：

- `Server.MirDB`
- `Configs/Setup.ini`

若全部找不到，则报错并列出尝试过的路径：

- `错误：找不到有效的服务器目录（包含 Server.MirDB 和 Configs/Setup.ini）。`

#### 2.1.4 输出内容

- 程序启动后会：
  - 设置 `Environment.CurrentDirectory = serverDir`，保证后续 `Settings.Load()` 使用相对路径；
  - 调用 `Envir.Main.LoadDB()` 加载数据库；
  - 输出各类对象数量（地图、物品、怪物、NPC、任务、技能等）。

- 所有导出 CSV 文件输出到：

  ```text
  docs/translations/
  ```

- 具体文件及列结构：

  - **items_translation.csv**
    - 头：`ID;Index;EnglishName;ChineseName;ItemType;Notes`
    - 数据行示例：
      - `Item_GoldOre;123;Gold Ore;;Ore;`
    - 约定：
      - `ID` = `Item_` + 物品名去除空格和括号，如 `GoldOre`。

  - **monsters_translation.csv**
    - 头：`ID;Index;EnglishName;ChineseName;Level;Notes`
    - 数据行示例：
      - `Monster_ChickenMonster;1;Chicken Monster;;1;`
    - 约定：
      - `ID` = `Monster_` + 怪物名去空格和减号。

  - **quests_translation.csv**
    - 头：`ID;Index;EnglishName;ChineseName;Group;Notes`
    - 数据行示例：
      - `Quest_Tutorial;1;Tutorial Quest;;Main;`

  - **magics_translation.csv**
    - 头：`ID;EnglishName;ChineseName;Spell;Notes`
    - 数据行示例：
      - `Magic_FireBall;Fire Ball;;43;`
    - 约定：
      - `ID` = `Magic_` + 技能名去空格和减号。

  - **npcs_translation.csv**
    - 头：`ID;Index;EnglishName;ChineseName;FileName;Notes`
    - 数据行示例：
      - `NPC_ShopKeeper;10;Shop Keeper;;ShopKeeper.txt;`

  - **maps_translation.csv**
    - 头：`ID;Index;FileName;EnglishName;ChineseName;Notes`
    - 数据行示例：
      - `Map_BichonProvince;0;0;Bichon Province;;`
    - 约定：
      - `ID` = `Map_` + `Title` 去空格和减号，和 `GameLanguage.GetMapName` 规则一致。

  - **quest_details_translation.csv**
    - 头：`QuestIndex;QuestName;Group;Type;LineIndex;EnglishText;ChineseText`
    - 用于任务描述/目标/完成文本等的逐行翻译，`Type` 标记来源（Description、TaskDescription、GotoMessage 等）。

> **重要约定**：翻译时只填写 `ChineseName`/`ChineseText` 列，不修改 `ID` / `EnglishName` / Index 等字段，以保持与数据库和 Language 系统的对应关系。

---

### 2.2 Tools.LanguageGenerator

#### 2.2.1 位置

- 工程路径：`Tools.LanguageGenerator/`
- 可执行文件（示例）：
  - `Tools.LanguageGenerator/bin/Debug/net8.0/Tools.LanguageGenerator.exe`

#### 2.2.2 模式与参数

源码中的参数解析逻辑：

```csharp
var mode = "generate";        // 默认模式
var configuration = "Debug";  // 默认配置名

if (args.Length > 0 && !string.IsNullOrWhiteSpace(args[0]))
{
    if (string.Equals(args[0], "merge", StringComparison.OrdinalIgnoreCase))
    {
        mode = "merge";
        if (args.Length > 1 && !string.IsNullOrWhiteSpace(args[1]))
            configuration = args[1];
    }
    else
    {
        configuration = args[0];
    }
}
```

因此：

- **无参数**：
  - `mode = generate`，`configuration = "Debug"`；
- **第一个参数是 `merge`（不带 `-`）**：
  - `mode = merge`，第二个参数若存在则当作 configuration（目前 merge 模式中实际上未使用）；
- **第一个参数是其它字符串（如 `Release`）**：
  - `mode = generate`，`configuration` 取该字符串。

> 注意：`-merge`（带横杠）会被当成配置名，而不是 merge 模式。

#### 2.2.3 默认模式：生成 Language.ini

- **命令格式**：

  ```powershell
  Tools.LanguageGenerator.exe [<Configuration>]
  ```

- 行为：
  - 读取：`Shared/Languages.csv`；
  - 将每一行解析为 `(Key;Scope;en;zh-CN)`，并分成：
    - Client 端条目：`Scope in {Client, Both}`；
    - Server 端条目：`Scope in {Server, Both}`；
  - 输出：
    - `Build/Client/<Configuration>/Language.ini`
    - `Build/Server/<Configuration>/Configs/Language.ini`
  - INI 格式：

    ```ini
    [Language]
    Key=值
    ```

    其中 `值` 为：
    - 若 `zh-CN` 非空，用中文；
    - 否则回退为英文 `en`。

- 示例：从仓库根目录运行：

  ```powershell
  # 生成 Debug 配置的 Language.ini
  .\Tools.LanguageGenerator\bin\Debug\net8.0\Tools.LanguageGenerator.exe

  # 生成 Release 配置的 Language.ini
  .\Tools.LanguageGenerator\bin\Debug\net8.0\Tools.LanguageGenerator.exe Release
  ```

#### 2.2.4 merge 模式：合并翻译 CSV 到 Languages.csv

- **命令格式**：

  ```powershell
  Tools.LanguageGenerator.exe merge [<Configuration>]
  ```

  （当前 `<Configuration>` 参数在 merge 模式中未使用，可忽略。）

- 行为：
  - 读取：
    - `Shared/Languages.csv`
    - `docs/translations/` 目录下若干翻译 CSV：
      - `items_translation.csv`
      - `monsters_translation.csv`
      - `npcs_translation.csv`
      - `magics_translation.csv`
      - `quests_translation.csv`
      - `maps_translation.csv`
  - 将每个 CSV 中 **有中文的行** 按 ID 合并到字典 `entries`：

    ```text
    Key -> (Scope, En, Zh)
    ```

  - 每个文件对应的列索引（从 0 开始）：
    - `items_translation.csv`：`idIndex=0, enIndex=2, zhIndex=3`
    - `monsters_translation.csv`：`0, 2, 3`
    - `npcs_translation.csv`：`0, 2, 3`
    - `magics_translation.csv`：`0, 1, 2`
    - `quests_translation.csv`：`0, 2, 3`
    - `maps_translation.csv`：`0, 3, 4`

  - 合并策略：
    - 若 `Languages.csv` 中不存在该 Key：
      - 新建：`Scope = "Both"`，`en = <CSV 中英文名>`，`zh = <CSV 中中文名>`；
    - 若已存在：
      - 若原 `zh` 为空：使用新的中文翻译；
      - 若原 `zh` 非空且与新翻译不同：
        - **控制台输出覆盖日志**：
          - `Overwrite zh-CN for <ID>: "旧" -> "新"`
        - 并采用新的翻译覆盖旧值。

  - 最终写回：
    - 重新生成 `Shared/Languages.csv`：
      - 按 Key 字母顺序排序输出；
      - 头：`Key;Scope;en;zh-CN`；
      - 确保 `en`/`zh` 中去掉换行，Scope 为空时回退为 `Both`。

- 示例：从仓库根目录运行：

  ```powershell
  .\Tools.LanguageGenerator\bin\Debug\net8.0\Tools.LanguageGenerator.exe merge
  ```

---

## 三、推荐工作流：数据库汉化 & 语言文件生成

结合上面的工具和 `docs/数据库汉化方案.md`，推荐的日常工作流程如下：

### Step 1：确保服务器构建

- 先用 `dotnet build` 或现有脚本，确保对应配置下存在：
  - `Build/Server/Debug` 或 `Build/Server/Release` 目录；
  - 目录内包含最新的 `Server.MirDB` 和 `Configs/Setup.ini`。

### Step 2：从数据库导出翻译骨架

- 从仓库根目录执行：

  ```powershell
  # Debug 配置
  .\Tools.DatabaseExporter\bin\Debug\net8.0\Tools.DatabaseExporter.exe

  # 或 Release 配置
  .\Tools.DatabaseExporter\bin\Debug\net8.0\Tools.DatabaseExporter.exe Release
  ```

- 结果：
  - 在 `docs/translations/` 下生成/覆盖若干 `*_translation.csv` 文件；
  - 这些 CSV 只包含英文名和空的中文列，作为翻译模板。

### Step 3：填写翻译

- 使用 Excel / 表格工具打开 `docs/translations/*.csv`：
  - 只填写 `ChineseName` / `ChineseText` 列；
  - 不要修改 `ID`、`Index`、`EnglishName` 等列；
  - 可在 `Notes` 列写一些备注（掉落出处、任务链等）。

### Step 4：合并到 Languages.csv

- 完成 CSV 翻译后，从仓库根目录执行：

  ```powershell
  .\Tools.LanguageGenerator\bin\Debug\net8.0\Tools.LanguageGenerator.exe merge
  ```

- 程序会：
  - 把有中文的行合并到 `Shared/Languages.csv`；
  - 如遇已有翻译且内容不同，会在控制台打印覆盖信息，便于人工检查。

### Step 5：生成客户端/服务端 Language.ini

- 对需要运行的配置生成语言文件，例如：

  ```powershell
  # 为 Debug 配置生成 Language.ini
  .\Tools.LanguageGenerator\bin\Debug\net8.0\Tools.LanguageGenerator.exe

  # 为 Release 配置生成 Language.ini
  .\Tools.LanguageGenerator\bin\Debug\net8.0\Tools.LanguageGenerator.exe Release
  ```

- 生成文件：
  - 客户端：`Build/Client/<Configuration>/Language.ini`
  - 服务器：`Build/Server/<Configuration>/Configs/Language.ini`

### Step 6：启动游戏验证

- 启动服务器（C# 或 Rust，根据环境）；
- 启动客户端：
  - 检查物品名、怪物名、技能名、任务名等是否按预期显示中文；
  - 检查系统消息、地图名等是否正常；
- 对于 GM 命令：
  - 使用 `/gm` 开启 GM 模式；
  - 测试 `/level`、`/skill`、`/guildexp`、`/kill` 等命令是否行为和预期一致。

---

## 四、附录：常用命令 & 工具速查表

### 4.1 GM 命令速查

- `/gm` / `/gm off`：开启/关闭 GM 模式；
- `/addstorage`：购买/续期扩展仓库（1,000,000 Gold / 10 天，账号共享）；
- `/guildexp <amount>`：为当前所在公会增加经验（GM 仅限）；
- `/createguild <name>`：创建新公会，受等级与名称规则限制；
- `/showmemoney <delta>`：为当前角色增加金币（GM 仅限，测试用途）；
- `/level <level>`：设置角色等级，自动重算 HP/MP 和经验（GM 仅限）；
- `/skill`：为当前职业学习所有职业技能（GM 仅限）；
- `/skill <id|name>`：根据技能 ID 或名称学习单个技能（GM 仅限）；
- `/kill`：杀死当前位置最近的怪物并正常结算经验（GM 仅限）。

### 4.2 工具命令速查

- **DatabaseExporter**：

  ```powershell
  # Debug
  .\Tools.DatabaseExporter\bin\Debug\net8.0\Tools.DatabaseExporter.exe

  # Release
  .\Tools.DatabaseExporter\bin\Debug\net8.0\Tools.DatabaseExporter.exe Release
  ```

- **LanguageGenerator**：

  ```powershell
  # 合并 CSV 翻译到 Languages.csv
  .\Tools.LanguageGenerator\bin\Debug\net8.0\Tools.LanguageGenerator.exe merge

  # 生成 Debug 配置 Language.ini
  .\Tools.LanguageGenerator\bin\Debug\net8.0\Tools.LanguageGenerator.exe

  # 生成 Release 配置 Language.ini
  .\Tools.LanguageGenerator\bin\Debug\net8.0\Tools.LanguageGenerator.exe Release
  ```

> 后续如果新增新的运维工具或 GM 命令，可直接在本文件中补充对应章节，保持整套运维文档的一致性。

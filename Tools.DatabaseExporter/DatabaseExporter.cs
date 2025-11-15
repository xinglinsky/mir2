// DatabaseExporter: 从服务器数据库导出名称列表（怪物、物品、技能、任务等）到 CSV
// 仅做“从数据库导出”，不生成任何翻译内容。

using System.Text;
using Server;
using Server.MirEnvir;
using Server.MirDatabase;

Console.WriteLine("=== 传奇2 数据库导出工具 ===");
Console.WriteLine();

// 可选参数：Debug / Release，对应 Build/Server/<Configuration>
var configuration = (args.Length > 0 && !string.IsNullOrWhiteSpace(args[0]))
    ? args[0]
    : "Debug";

// 解决方案根目录（mir2）
var baseDir = AppContext.BaseDirectory;
var solutionRoot = Path.GetFullPath(Path.Combine(baseDir, "..", "..", "..", ".."));

// 服务器工作目录候选（必须包含 Server.MirDB 和 Configs/Setup.ini）
var serverCandidates = new[]
{
    Path.Combine(solutionRoot, "Build", "Server", configuration),
    Path.Combine(solutionRoot, "Build", "Server", "Debug"),
    Path.Combine(solutionRoot, "Build", "Server", "Release")
};

string? serverDir = null;
string? dbPath = null;

foreach (var dir in serverCandidates)
{
    var candidateDb = Path.Combine(dir, "Server.MirDB");
    var candidateSetup = Path.Combine(dir, "Configs", "Setup.ini");
    if (File.Exists(candidateDb) && File.Exists(candidateSetup))
    {
        serverDir = dir;
        dbPath = candidateDb;
        break;
    }
}

if (serverDir == null || dbPath == null)
{
    Console.Error.WriteLine("错误：找不到有效的服务器目录（包含 Server.MirDB 和 Configs/Setup.ini）。");
    Console.WriteLine("尝试的路径：");
    foreach (var dir in serverCandidates)
    {
        Console.WriteLine($"  - {dir}");
    }
    return 1;
}

Console.WriteLine($"使用服务器工作目录: {serverDir}");
Console.WriteLine($"数据库文件: {dbPath}");
Console.WriteLine();

// 切换当前工作目录，让 Settings/Envir 使用正确的相对路径
Environment.CurrentDirectory = serverDir;

try
{
    // 加载服务器配置（Setup.ini 等）
    Console.WriteLine("加载 Settings...");
    Settings.Load();

    // 加载数据库
    Console.WriteLine("加载 Server.MirDB...");
    var envir = Envir.Main;
    if (!envir.LoadDB())
    {
        Console.Error.WriteLine("错误：Envir.LoadDB() 返回 false，无法读取数据库。");
        return 1;
    }

    Console.WriteLine();
    Console.WriteLine($"物品数量:   {envir.ItemInfoList.Count}");
    Console.WriteLine($"怪物数量:   {envir.MonsterInfoList.Count}");
    Console.WriteLine($"NPC 数量:   {envir.NPCInfoList.Count}");
    Console.WriteLine($"任务数量:   {envir.QuestInfoList.Count}");
    Console.WriteLine($"技能数量:   {envir.MagicInfoList.Count}");
    Console.WriteLine();

    // 输出目录：docs/translations
    var outputDir = Path.Combine(solutionRoot, "docs", "translations");
    Directory.CreateDirectory(outputDir);

    // 准备导出数据（只包含英文名称，不包含任何翻译）
    var itemTuples = envir.ItemInfoList
        .Select(i => (Index: i.Index, Name: i.Name, Type: i.Type.ToString()))
        .ToList();

    var monsterTuples = envir.MonsterInfoList
        .Select(m => (Index: m.Index, Name: m.Name, Level: (int)m.Level))
        .ToList();

    var questTuples = envir.QuestInfoList
        .Select(q => (Index: q.Index, Name: q.Name, Group: q.Group))
        .ToList();

    var magicTuples = envir.MagicInfoList
        .Select(m => (Name: m.Name, Spell: m.Spell.ToString()))
        .ToList();

    var npcTuples = envir.NPCInfoList
        .Select(n => (Index: n.Index, Name: n.Name, FileName: n.FileName))
        .ToList();

    // 先导出任务详情（描述文本等）
    ExportQuestDetails(outputDir, envir.QuestInfoList);

    // 再导出名称列表
    ExportToCsv(outputDir, itemTuples, monsterTuples, questTuples, magicTuples, npcTuples);

    Console.WriteLine();
    Console.WriteLine("导出完成！");
    Console.WriteLine($"输出目录: {outputDir}");

    return 0;
}
catch (Exception ex)
{
    Console.Error.WriteLine($"错误: {ex.Message}");
    Console.Error.WriteLine(ex.StackTrace);
    return 1;
}

// ===== 辅助方法 =====

static void ExportToCsv(
    string outputDir,
    List<(int Index, string Name, string Type)> items,
    List<(int Index, string Name, int Level)> monsters,
    List<(int Index, string Name, string Group)> quests,
    List<(string Name, string Spell)> magics,
    List<(int Index, string Name, string FileName)> npcs)
{
    // 导出物品
    var itemsCsv = Path.Combine(outputDir, "items_translation.csv");
    using (var writer = new StreamWriter(itemsCsv, false, Encoding.UTF8))
    {
        writer.WriteLine("ID;Index;EnglishName;ChineseName;ItemType;Notes");
        foreach (var item in items.OrderBy(x => x.Type).ThenBy(x => x.Index))
        {
            var id = item.Name.Replace(" ", "").Replace("(", "").Replace(")", "");
            writer.WriteLine($"Item_{id};{item.Index};{item.Name};;{item.Type};");
        }
    }
    Console.WriteLine($"✓ 导出 {items.Count} 个物品到: {itemsCsv}");

    // 导出怪物
    var monstersCsv = Path.Combine(outputDir, "monsters_translation.csv");
    using (var writer = new StreamWriter(monstersCsv, false, Encoding.UTF8))
    {
        writer.WriteLine("ID;Index;EnglishName;ChineseName;Level;Notes");
        foreach (var monster in monsters.OrderBy(x => x.Level).ThenBy(x => x.Name))
        {
            var id = monster.Name.Replace(" ", "").Replace("-", "");
            writer.WriteLine($"Monster_{id};{monster.Index};{monster.Name};;{monster.Level};");
        }
    }
    Console.WriteLine($"✓ 导出 {monsters.Count} 个怪物到: {monstersCsv}");

    // 导出任务
    var questsCsv = Path.Combine(outputDir, "quests_translation.csv");
    using (var writer = new StreamWriter(questsCsv, false, Encoding.UTF8))
    {
        writer.WriteLine("ID;Index;EnglishName;ChineseName;Group;Notes");
        foreach (var quest in quests.OrderBy(x => x.Index))
        {
            var id = quest.Name.Replace(" ", "").Replace("-", "");
            writer.WriteLine($"Quest_{id};{quest.Index};{quest.Name};;{quest.Group};");
        }
    }
    Console.WriteLine($"✓ 导出 {quests.Count} 个任务到: {questsCsv}");

    // 导出技能
    var magicsCsv = Path.Combine(outputDir, "magics_translation.csv");
    using (var writer = new StreamWriter(magicsCsv, false, Encoding.UTF8))
    {
        writer.WriteLine("ID;EnglishName;ChineseName;Spell;Notes");
        foreach (var magic in magics.OrderBy(x => x.Spell))
        {
            var id = magic.Name.Replace(" ", "").Replace("-", "");
            writer.WriteLine($"Magic_{id};{magic.Name};;{magic.Spell};");
        }
    }
    Console.WriteLine($"✓ 导出 {magics.Count} 个技能到: {magicsCsv}");

    // 导出 NPC
    var npcsCsv = Path.Combine(outputDir, "npcs_translation.csv");
    using (var writer = new StreamWriter(npcsCsv, false, Encoding.UTF8))
    {
        writer.WriteLine("ID;Index;EnglishName;ChineseName;FileName;Notes");
        foreach (var npc in npcs.OrderBy(x => x.Name))
        {
            var id = npc.Name.Replace(" ", "").Replace("-", "");
            writer.WriteLine($"NPC_{id};{npc.Index};{npc.Name};;{npc.FileName};");
        }
    }
    Console.WriteLine($"✓ 导出 {npcs.Count} 个 NPC 到: {npcsCsv}");
}

static void ExportQuestDetails(string outputDir, IList<QuestInfo> quests)
{
    var detailsCsv = Path.Combine(outputDir, "quest_details_translation.csv");
    using (var writer = new StreamWriter(detailsCsv, false, Encoding.UTF8))
    {
        writer.WriteLine("QuestIndex;QuestName;Group;Type;LineIndex;EnglishText;ChineseText");

        foreach (var q in quests.OrderBy(x => x.Index))
        {
            WriteLines("Description", q.Description);
            WriteLines("TaskDescription", q.TaskDescription);
            WriteLines("ReturnDescription", q.ReturnDescription);
            WriteLines("CompletionDescription", q.CompletionDescription);

            WriteSingle("GotoMessage", q.GotoMessage);
            WriteSingle("KillMessage", q.KillMessage);
            WriteSingle("ItemMessage", q.ItemMessage);
            WriteSingle("FlagMessage", q.FlagMessage);

            void WriteLines(string type, IList<string> lines)
            {
                if (lines == null) return;
                for (int i = 0; i < lines.Count; i++)
                {
                    var text = SanitizeCsvField(lines[i]);
                    writer.WriteLine($"{q.Index};{SanitizeCsvField(q.Name)};{SanitizeCsvField(q.Group)};{type};{i};{text};");
                }
            }

            void WriteSingle(string type, string text)
            {
                if (string.IsNullOrWhiteSpace(text)) return;
                var t = SanitizeCsvField(text);
                writer.WriteLine($"{q.Index};{SanitizeCsvField(q.Name)};{SanitizeCsvField(q.Group)};{type};0;{t};");
            }
        }
    }
    Console.WriteLine($"✓ 导出 {quests.Count} 个任务详情到: {detailsCsv}");
}

static string SanitizeCsvField(string value)
{
    if (string.IsNullOrEmpty(value)) return string.Empty;
    return value
        .Replace("\r", " ")
        .Replace("\n", " ")
        .Replace(";", "；");
}

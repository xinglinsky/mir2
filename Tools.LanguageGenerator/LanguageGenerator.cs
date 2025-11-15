// See https://aka.ms/new-console-template for more information
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;

// LanguageGenerator:
// 1) 默认模式：从 Shared/Languages.csv 生成 Client/Server 的 Language.ini。
// 2) merge 模式：从 docs/translations/*.csv 合并翻译到 Shared/Languages.csv（填充 zh-CN 列）。

var mode = "generate";
var configuration = "Debug";

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

var baseDir = AppContext.BaseDirectory;
// baseDir = Tools.LanguageGenerator/bin/<Config>/net8.0-windows*/
// go up four levels to reach solution root (mir2).
var solutionRoot = Path.GetFullPath(Path.Combine(baseDir, "..", "..", "..", ".."));

var csvPath = Path.Combine(solutionRoot, "Shared", "Languages.csv");

if (!File.Exists(csvPath))
{
    Console.Error.WriteLine("Languages.csv not found. Expected at: " + csvPath);
    return;
}

if (string.Equals(mode, "merge", StringComparison.OrdinalIgnoreCase))
{
    var translationsDir = Path.Combine(solutionRoot, "docs", "translations");
    Console.WriteLine($"Languages.csv: {csvPath}");
    Console.WriteLine($"Translations dir: {translationsDir}");
    MergeTranslations(csvPath, translationsDir);
    Console.WriteLine("Merge completed.");
    return;
}

var clientIniPath = Path.Combine(solutionRoot, "Build", "Client", configuration, "Language.ini");
var serverIniPath = Path.Combine(solutionRoot, "Build", "Server", configuration, "Configs", "Language.ini");

Console.WriteLine($"CSV: {csvPath}");
Console.WriteLine($"Client INI: {clientIniPath}");
Console.WriteLine($"Server INI: {serverIniPath}");

var lines = File.ReadAllLines(csvPath, Encoding.UTF8);
if (lines.Length <= 1)
{
    Console.Error.WriteLine("Languages.csv is empty or only has header.");
    return;
}

var entries = new List<(string Key, string Scope, string En, string Zh)>();

for (int i = 1; i < lines.Length; i++)
{
    var line = lines[i].Trim();
    if (string.IsNullOrWhiteSpace(line)) continue;

    var parts = line.Split(';');
    if (parts.Length < 4) continue;

    var key = parts[0].Trim();
    var scope = parts[1].Trim();
    var en = parts[2];
    var zh = parts[3];

    entries.Add((key, scope, en, zh));
}

var clientEntries = entries
    .Where(e => string.Equals(e.Scope, "Client", StringComparison.OrdinalIgnoreCase) ||
                string.Equals(e.Scope, "Both", StringComparison.OrdinalIgnoreCase))
    .ToList();

var serverEntries = entries
    .Where(e => string.Equals(e.Scope, "Server", StringComparison.OrdinalIgnoreCase) ||
                string.Equals(e.Scope, "Both", StringComparison.OrdinalIgnoreCase))
    .ToList();

Directory.CreateDirectory(Path.GetDirectoryName(clientIniPath)!);
using (var writer = new StreamWriter(clientIniPath, false, Encoding.UTF8))
{
    writer.WriteLine("[Language]");
    foreach (var e in clientEntries)
    {
        var value = !string.IsNullOrWhiteSpace(e.Zh) ? e.Zh : e.En;
        writer.WriteLine($"{e.Key}={value}");
    }
}

Directory.CreateDirectory(Path.GetDirectoryName(serverIniPath)!);
using (var writer = new StreamWriter(serverIniPath, false, Encoding.UTF8))
{
    writer.WriteLine("[Language]");
    foreach (var e in serverEntries)
    {
        var value = !string.IsNullOrWhiteSpace(e.Zh) ? e.Zh : e.En;
        writer.WriteLine($"{e.Key}={value}");
    }
}

Console.WriteLine($"Generated Client Language.ini with {clientEntries.Count} entries.");
Console.WriteLine($"Generated Server Language.ini with {serverEntries.Count} entries.");

static void MergeTranslations(string csvPath, string translationsDir)
{
    if (!Directory.Exists(translationsDir))
    {
        Console.Error.WriteLine("Translations directory not found: " + translationsDir);
        return;
    }

    var languageLines = File.ReadAllLines(csvPath, Encoding.UTF8);
    if (languageLines.Length == 0)
    {
        Console.Error.WriteLine("Languages.csv is empty.");
        return;
    }

    // 解析现有 Languages.csv 为字典（Key -> Scope/En/Zh）
    var entries = new Dictionary<string, (string Scope, string En, string Zh)>(StringComparer.OrdinalIgnoreCase);

    for (int i = 1; i < languageLines.Length; i++)
    {
        var line = languageLines[i];
        if (string.IsNullOrWhiteSpace(line)) continue;

        var parts = line.Split(';');
        if (parts.Length < 4) continue;

        var key = parts[0].Trim();
        var scope = parts[1].Trim();
        var en = parts[2];
        var zh = parts[3];

        if (string.IsNullOrWhiteSpace(key)) continue;

        if (!entries.ContainsKey(key))
            entries[key] = (scope, en, zh);
    }

    void Apply(string fileName, int idIndex, int enIndex, int zhIndex)
    {
        var path = Path.Combine(translationsDir, fileName);
        if (!File.Exists(path))
        {
            Console.WriteLine($"Translation file not found: {path} (skipped)");
            return;
        }

        Console.WriteLine($"Merging {fileName} ...");

        var lines = File.ReadAllLines(path, Encoding.UTF8);
        if (lines.Length <= 1) return;

        for (int i = 1; i < lines.Length; i++)
        {
            var line = lines[i];
            if (string.IsNullOrWhiteSpace(line)) continue;

            var parts = line.Split(';');
            if (parts.Length <= Math.Max(zhIndex, Math.Max(idIndex, enIndex))) continue;

            var id = parts[idIndex].Trim();
            var en = parts[enIndex];
            var zh = parts[zhIndex];

            if (string.IsNullOrWhiteSpace(id) || string.IsNullOrWhiteSpace(zh))
                continue;

            if (!entries.TryGetValue(id, out var existing))
            {
                entries[id] = ("Both", en, zh);
            }
            else
            {
                if (string.IsNullOrWhiteSpace(existing.Zh))
                {
                    entries[id] = (existing.Scope, existing.En, zh);
                }
                else if (!string.Equals(existing.Zh, zh, StringComparison.Ordinal))
                {
                    // 默认策略：覆盖为最新翻译，并打印提示。
                    Console.WriteLine($"  Overwrite zh-CN for {id}: \"{existing.Zh}\" -> \"{zh}\"");
                    entries[id] = (existing.Scope, existing.En, zh);
                }
            }
        }
    }

    Apply("items_translation.csv", 0, 2, 3);
    Apply("monsters_translation.csv", 0, 2, 3);
    Apply("npcs_translation.csv", 0, 2, 3);
    Apply("magics_translation.csv", 0, 1, 2);
    Apply("quests_translation.csv", 0, 2, 3);

    // 写回 Languages.csv（简单按 Key 排序，保证稳定输出）
    var sb = new StringBuilder();
    sb.AppendLine("Key;Scope;en;zh-CN");

    foreach (var pair in entries.OrderBy(e => e.Key, StringComparer.OrdinalIgnoreCase))
    {
        var key = pair.Key;
        var (scope, en, zh) = pair.Value;

        en ??= string.Empty;
        zh ??= string.Empty;

        en = en.Replace("\r", "").Replace("\n", " ");
        zh = zh.Replace("\r", "").Replace("\n", " ");

        sb.Append(key).Append(';')
          .Append(string.IsNullOrWhiteSpace(scope) ? "Both" : scope).Append(';')
          .Append(en).Append(';')
          .Append(zh).AppendLine();
    }

    File.WriteAllText(csvPath, sb.ToString(), Encoding.UTF8);
}

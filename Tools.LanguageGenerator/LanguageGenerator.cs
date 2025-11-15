// See https://aka.ms/new-console-template for more information
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;

// LanguageGenerator: generate Client Language.ini from Shared/Languages.csv
// Run from anywhere; it will resolve paths based on the compiled assembly location.
var configuration = (args.Length > 0 && !string.IsNullOrWhiteSpace(args[0]))
    ? args[0]
    : "Debug";

var baseDir = AppContext.BaseDirectory;
// baseDir = Tools.LanguageGenerator/bin/<Config>/net8.0-windows*/
// go up four levels to reach solution root (mir2).
var solutionRoot = Path.GetFullPath(Path.Combine(baseDir, "..", "..", "..", ".."));

var csvPath = Path.Combine(solutionRoot, "Shared", "Languages.csv");
var clientIniPath = Path.Combine(solutionRoot, "Build", "Client", configuration, "Language.ini");
var serverIniPath = Path.Combine(solutionRoot, "Build", "Server", configuration, "Configs", "Language.ini");

Console.WriteLine($"CSV: {csvPath}");
Console.WriteLine($"Client INI: {clientIniPath}");
Console.WriteLine($"Server INI: {serverIniPath}");

if (!File.Exists(csvPath))
{
    Console.Error.WriteLine("Languages.csv not found. Expected at: " + csvPath);
    return;
}

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

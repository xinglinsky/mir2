using System.Text.Json;
using Server;
using Server.MirDatabase;
using Server.MirEnvir;

internal static class Program
{
    private static int Main(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("missing command");
            return 1;
        }

        var cmd = args[0];
        var rest = args.Skip(1).ToArray();

        try
        {
            return cmd switch
            {
                "drops" => RunDrops(rest),
                "npc-expand" => RunNpcExpand(rest),
                "quest" => RunQuest(rest),
                "routes" => RunRoutes(rest),
                "recipe" => RunRecipe(rest),
                "export-mapinfos" => RunExportMapInfos(rest),
                "export-iteminfos" => RunExportItemInfos(rest),
                "export-monsterinfos" => RunExportMonsterInfos(rest),
                "export-npcinfos" => RunExportNpcInfos(rest),
                "export-questinfos" => RunExportQuestInfos(rest),
                "export-magicinfos" => RunExportMagicInfos(rest),
                "export-gameshopitems" => RunExportGameShopItems(rest),
                "export-conquestinfos" => RunExportConquestInfos(rest),
                _ => throw new Exception($"unknown command: {cmd}"),
            };
        }
        catch (Exception e)
        {
            Console.Error.WriteLine(e.ToString());
            return 2;
        }
    }

    private static int RunDrops(string[] args)
    {
        string root = null;
        string dropFile = null;
        string outPath = null;
        byte dropType = 0;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--drop") dropFile = args[++i];
            else if (a == "--out") outPath = args[++i];
            else if (a == "--type") dropType = byte.Parse(args[++i]);
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(dropFile)) throw new Exception("--drop is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        Directory.SetCurrentDirectory(root);

        if (!Envir.Main.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Main.LoadDB()");
        }

        var full = Path.Combine(Settings.DropPath, dropFile);

        var list = new List<DropInfo>();
        DropInfo.Load(list, dropFile, full, dropType, createIfNotExists: false);

        var fixture = new DropsFixture
        {
            drop_path = NormalizePath(dropFile),
            drop_type = dropType,
            drops = list.Select(FromDrop).ToList()
        };

        var json = JsonSerializer.Serialize(fixture, new JsonSerializerOptions { WriteIndented = true });
        File.WriteAllText(outPath, json);
        return 0;
    }

    private static void ApplyQuestDetailTranslations(QuestInfo quest, string mir2RootOverride)
    {
        if (quest == null) return;

        string root = mir2RootOverride;
        if (string.IsNullOrWhiteSpace(root))
        {
            root = FindMir2RootFromBaseDir();
        }

        if (string.IsNullOrWhiteSpace(root)) return;

        string csvPath = Path.Combine(root, "docs", "translations", "quest_details_translation.csv");
        if (!File.Exists(csvPath)) return;

        Dictionary<Tuple<int, string, int>, string> lines;
        try
        {
            lines = LoadQuestDetailTranslationMap(csvPath);
        }
        catch
        {
            return;
        }

        if (lines.Count == 0) return;

        ReplaceList(lines, quest.Index, "Description", quest.Description);
        ReplaceList(lines, quest.Index, "TaskDescription", quest.TaskDescription);
        ReplaceList(lines, quest.Index, "ReturnDescription", quest.ReturnDescription);
        ReplaceList(lines, quest.Index, "CompletionDescription", quest.CompletionDescription);

        quest.GotoMessage = ReplaceSingle(lines, quest.Index, "GotoMessage", quest.GotoMessage);
        quest.KillMessage = ReplaceSingle(lines, quest.Index, "KillMessage", quest.KillMessage);
        quest.ItemMessage = ReplaceSingle(lines, quest.Index, "ItemMessage", quest.ItemMessage);
        quest.FlagMessage = ReplaceSingle(lines, quest.Index, "FlagMessage", quest.FlagMessage);
    }

    private static string FindMir2RootFromBaseDir()
    {
        var dir = new DirectoryInfo(AppDomain.CurrentDomain.BaseDirectory);
        for (int i = 0; i < 8 && dir != null; i++)
        {
            var csv = Path.Combine(dir.FullName, "docs", "translations", "quest_details_translation.csv");
            if (File.Exists(csv))
            {
                return dir.FullName;
            }
            dir = dir.Parent;
        }
        return null;
    }

    private static Dictionary<Tuple<int, string, int>, string> LoadQuestDetailTranslationMap(string csvPath)
    {
        var outMap = new Dictionary<Tuple<int, string, int>, string>();
        var allLines = File.ReadAllLines(csvPath, System.Text.Encoding.UTF8);
        if (allLines.Length <= 1) return outMap;

        for (int i = 1; i < allLines.Length; i++)
        {
            var line = allLines[i];
            if (string.IsNullOrWhiteSpace(line)) continue;

            var parts = line.Split(';');
            if (parts.Length < 7) continue;

            if (!int.TryParse(parts[0], out int questIndex)) continue;
            var type = parts[3].Trim();
            if (!int.TryParse(parts[4], out int lineIndex)) lineIndex = 0;
            var chinese = parts[6];
            if (string.IsNullOrWhiteSpace(chinese)) continue;

            var key = Tuple.Create(questIndex, type, lineIndex);
            outMap[key] = chinese;
        }

        return outMap;
    }

    private static void ReplaceList(
        Dictionary<Tuple<int, string, int>, string> map,
        int questIndex,
        string type,
        List<string> list)
    {
        if (list == null || list.Count == 0) return;

        for (int i = 0; i < list.Count; i++)
        {
            var key = Tuple.Create(questIndex, type, i);
            if (map.TryGetValue(key, out string value) && !string.IsNullOrWhiteSpace(value))
            {
                list[i] = value;
            }
        }
    }

    private static string ReplaceSingle(
        Dictionary<Tuple<int, string, int>, string> map,
        int questIndex,
        string type,
        string current)
    {
        if (string.IsNullOrEmpty(current)) return current;
        var key = Tuple.Create(questIndex, type, 0);
        if (map.TryGetValue(key, out string value) && !string.IsNullOrWhiteSpace(value))
        {
            return value;
        }
        return current;
    }

    private static string NormalizePath(string s)
    {
        return s.Replace('\\', '/');
    }

    private static DropNode FromDrop(DropInfo d)
    {
        return new DropNode
        {
            chance = d.Chance,
            gold = d.Gold,
            item_index = d.Item?.Index,
            item_name = d.Item?.Name,
            item_type = d.Item == null ? null : (int?)d.Item.Type,
            quest_required = d.QuestRequired,
            drop_type = d.Type,
            group = d.GroupedDrop == null ? null : new GroupNode
            {
                random = d.GroupedDrop.Random,
                first = d.GroupedDrop.First,
                drops = d.GroupedDrop.Select(FromDrop).ToList()
            }
        };
    }

    private class DropsFixture
    {
        public string drop_path { get; set; }
        public byte drop_type { get; set; }
        public List<DropNode> drops { get; set; }
    }

    private class DropNode
    {
        public int chance { get; set; }
        public uint gold { get; set; }
        public int? item_index { get; set; }
        public string item_name { get; set; }
        public int? item_type { get; set; }
        public bool quest_required { get; set; }
        public byte drop_type { get; set; }
        public GroupNode group { get; set; }
    }

    private class GroupNode
    {
        public bool random { get; set; }
        public bool first { get; set; }
        public List<DropNode> drops { get; set; }
    }

    private static int RunNpcExpand(string[] args)
    {
        string root = null;
        string npcFile = null;
        string outPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--npc") npcFile = args[++i];
            else if (a == "--out") outPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(npcFile)) throw new Exception("--npc is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        Directory.SetCurrentDirectory(root);

        var npcPath = Path.Combine(Settings.NPCPath, npcFile);
        if (!File.Exists(npcPath))
        {
            throw new Exception($"npc script not found: {npcPath}");
        }

        var lines = File.ReadAllLines(npcPath).ToList();
        lines = NpcParseInsert(lines);
        lines = NpcParseInclude(lines);

        var fixture = new NpcExpandFixture
        {
            npc_file = NormalizePath(npcFile),
            expanded_lines = lines,
        };

        var json = JsonSerializer.Serialize(fixture, new JsonSerializerOptions { WriteIndented = true });
        File.WriteAllText(outPath, json);
        return 0;
    }

    private static List<string> NpcParseInsert(List<string> lines)
    {
        List<string> newLines = new List<string>();

        for (int i = 0; i < lines.Count; i++)
        {
            if (!lines[i].ToUpper().StartsWith("#INSERT")) continue;

            string[] split = lines[i].Split(' ');
            if (split.Length < 2) continue;

            string token = split[1];
            if (token.Length < 2) continue;
            string sub = token.Substring(1, token.Length - 2);

            string path = Path.Combine(Settings.EnvirPath, sub);

            if (!File.Exists(path))
                MessageQueue.Instance.Enqueue(string.Format("INSERT Script Not Found: {0}", path));
            else
                newLines = File.ReadAllLines(path).ToList();

            lines.AddRange(newLines);
        }

        lines.RemoveAll(str => str.ToUpper().StartsWith("#INSERT"));
        return lines;
    }

    private static List<string> NpcParseInclude(List<string> lines)
    {
        for (int i = 0; i < lines.Count; i++)
        {
            if (!lines[i].ToUpper().StartsWith("#INCLUDE")) continue;

            string[] split = lines[i].Split(' ');
            if (split.Length < 3) continue;

            string token = split[1];
            if (token.Length < 2) continue;
            string sub = token.Substring(1, token.Length - 2);

            string path = Path.Combine(Settings.EnvirPath, sub);
            string page = ("[" + split[2] + "]").ToUpper();

            bool start = false, finish = false;
            var parsedLines = new List<string>();

            if (!File.Exists(path))
            {
                MessageQueue.Instance.Enqueue(string.Format("INCLUDE Script Not Found: {0}", path));
                return parsedLines;
            }

            IList<string> extLines = File.ReadAllLines(path);

            for (int j = 0; j < extLines.Count; j++)
            {
                if (!extLines[j].ToUpper().StartsWith(page)) continue;

                for (int x = j + 1; x < extLines.Count; x++)
                {
                    if (extLines[x].Trim() == ("{"))
                    {
                        start = true;
                        continue;
                    }

                    if (extLines[x].Trim() == ("}"))
                    {
                        finish = true;
                        break;
                    }

                    parsedLines.Add(extLines[x]);
                }
            }

            if (start && finish)
            {
                lines.InsertRange(i + 1, parsedLines);
                parsedLines.Clear();
            }
        }

        lines.RemoveAll(str => str.ToUpper().StartsWith("#INCLUDE"));
        return lines;
    }

    private class NpcExpandFixture
    {
        public string npc_file { get; set; }
        public List<string> expanded_lines { get; set; }
    }

    private static int RunQuest(string[] args)
    {
        string root = null;
        string questFile = null;
        int? questIndex = null;
        string outPath = null;
        string mir2Root = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--file") questFile = args[++i];
            else if (a == "--index") questIndex = int.Parse(args[++i]);
            else if (a == "--out") outPath = args[++i];
            else if (a == "--mir2-root") mir2Root = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");
        if (string.IsNullOrWhiteSpace(questFile) && questIndex == null)
            throw new Exception("either --file or --index is required");

        Directory.SetCurrentDirectory(root);

        if (!Envir.Main.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Main.LoadDB()");
        }

        QuestInfo quest;
        if (!string.IsNullOrWhiteSpace(questFile))
        {
            var target = NormalizePath(questFile);
            quest = Envir.Main.QuestInfoList.FirstOrDefault(q =>
                NormalizePath(q.FileName + ".txt").Equals(target, StringComparison.OrdinalIgnoreCase));
        }
        else
        {
            quest = Envir.Main.GetQuestInfo(questIndex.Value);
        }

        if (quest == null)
        {
            throw new Exception($"quest not found (file={questFile ?? ""}, index={questIndex?.ToString() ?? ""})");
        }

        // Ensure we serialize the final parsed state (text sections + translations).
        quest.TaskDescription.Clear();
        quest.ReturnDescription.Clear();
        quest.CompletionDescription.Clear();
        quest.CarryItems.Clear();
        quest.LoadInfo(clear: true);

        ApplyQuestDetailTranslations(quest, mir2Root);

        var fixture = new QuestFixture
        {
            quest_index = quest.Index,
            quest_file = NormalizePath(quest.FileName + ".txt"),
            description = quest.Description.ToList(),
            task_description = quest.TaskDescription.ToList(),
            return_description = quest.ReturnDescription.ToList(),
            completion_description = quest.CompletionDescription.ToList(),
            carry_items = quest.CarryItems.Select(FromItemTask).ToList(),
            kill_tasks = quest.KillTasks.Select(FromKillTask).ToList(),
            item_tasks = quest.ItemTasks.Select(FromItemTask).ToList(),
            flag_tasks = quest.FlagTasks.Select(FromFlagTask).ToList(),
            fixed_rewards = quest.FixedRewards.Select(FromReward).ToList(),
            select_rewards = quest.SelectRewards.Select(FromReward).ToList(),
            gold_reward = quest.GoldReward,
            exp_reward = quest.ExpReward,
            credit_reward = quest.CreditReward,
        };

        var json = JsonSerializer.Serialize(fixture, new JsonSerializerOptions { WriteIndented = true });
        File.WriteAllText(outPath, json);
        return 0;
    }

    private static QuestKillTaskNode FromKillTask(QuestKillTask t)
    {
        return new QuestKillTaskNode
        {
            monster_index = t.Monster?.Index,
            monster_name = t.Monster?.Name,
            count = t.Count,
            message = t.Message,
        };
    }

    private static QuestItemTaskNode FromItemTask(QuestItemTask t)
    {
        return new QuestItemTaskNode
        {
            item_index = t.Item?.Index,
            item_name = t.Item?.Name,
            count = t.Count,
            message = t.Message,
        };
    }

    private static QuestFlagTaskNode FromFlagTask(QuestFlagTask t)
    {
        return new QuestFlagTaskNode
        {
            number = t.Number,
            message = t.Message,
        };
    }

    private static QuestItemRewardNode FromReward(QuestItemReward r)
    {
        return new QuestItemRewardNode
        {
            item_index = r.Item?.Index,
            item_name = r.Item?.Name,
            count = r.Count,
        };
    }

    private class QuestFixture
    {
        public int quest_index { get; set; }
        public string quest_file { get; set; }

        public List<string> description { get; set; }
        public List<string> task_description { get; set; }
        public List<string> return_description { get; set; }
        public List<string> completion_description { get; set; }

        public List<QuestItemTaskNode> carry_items { get; set; }
        public List<QuestKillTaskNode> kill_tasks { get; set; }
        public List<QuestItemTaskNode> item_tasks { get; set; }
        public List<QuestFlagTaskNode> flag_tasks { get; set; }
        public List<QuestItemRewardNode> fixed_rewards { get; set; }
        public List<QuestItemRewardNode> select_rewards { get; set; }

        public uint gold_reward { get; set; }
        public uint exp_reward { get; set; }
        public uint credit_reward { get; set; }
    }

    private class QuestKillTaskNode
    {
        public int? monster_index { get; set; }
        public string monster_name { get; set; }
        public int count { get; set; }
        public string message { get; set; }
    }

    private class QuestItemTaskNode
    {
        public int? item_index { get; set; }
        public string item_name { get; set; }
        public ushort count { get; set; }
        public string message { get; set; }
    }

    private class QuestFlagTaskNode
    {
        public int number { get; set; }
        public string message { get; set; }
    }

    private class QuestItemRewardNode
    {
        public int? item_index { get; set; }
        public string item_name { get; set; }
        public ushort count { get; set; }
    }

    private static int RunRoutes(string[] args)
    {
        string root = null;
        string routeFile = null;
        string outPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--route") routeFile = args[++i];
            else if (a == "--out") outPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(routeFile)) throw new Exception("--route is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        Directory.SetCurrentDirectory(root);

        var fileWithExt = routeFile.EndsWith(".txt", StringComparison.OrdinalIgnoreCase)
            ? routeFile
            : routeFile + ".txt";

        var full = Path.Combine(Settings.RoutePath, fileWithExt);
        if (!File.Exists(full))
        {
            throw new Exception($"route file not found: {full}");
        }

        var points = new List<RoutePointNode>();
        foreach (var line in File.ReadAllLines(full))
        {
            var info = RouteInfo.FromText(line);
            if (info == null) continue;
            points.Add(new RoutePointNode
            {
                x = info.Location.X,
                y = info.Location.Y,
                delay = info.Delay,
            });
        }

        var fixture = new RoutesFixture
        {
            route_file = NormalizePath(fileWithExt),
            points = points,
        };

        var json = JsonSerializer.Serialize(fixture, new JsonSerializerOptions { WriteIndented = true });
        File.WriteAllText(outPath, json);
        return 0;
    }

    private class RoutesFixture
    {
        public string route_file { get; set; }
        public List<RoutePointNode> points { get; set; }
    }

    private class RoutePointNode
    {
        public int x { get; set; }
        public int y { get; set; }
        public int delay { get; set; }
    }

    private static int RunRecipe(string[] args)
    {
        string root = null;
        string recipeFile = null;
        string outPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--recipe") recipeFile = args[++i];
            else if (a == "--out") outPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(recipeFile)) throw new Exception("--recipe is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        Directory.SetCurrentDirectory(root);

        if (!Envir.Main.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Main.LoadDB()");
        }

        var fileWithExt = recipeFile.EndsWith(".txt", StringComparison.OrdinalIgnoreCase)
            ? recipeFile
            : recipeFile + ".txt";

        var full = Path.Combine(Settings.RecipePath, fileWithExt);
        if (!File.Exists(full))
        {
            throw new Exception($"recipe file not found: {full}");
        }

        // Track which ingredients explicitly specify CurrentDura (3rd token).
        // This matches how Rust models current_dura as an Option.
        var ingredientDuraByIndex = new Dictionary<int, ushort>();
        {
            var lines = File.ReadAllLines(full);
            var mode = "ingredients";
            foreach (var raw in lines)
            {
                if (string.IsNullOrEmpty(raw)) continue;

                if (raw.StartsWith("["))
                {
                    mode = raw.Substring(1, raw.Length - 2).ToLower();
                    continue;
                }

                if (mode != "ingredients") continue;

                var data = raw.Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);
                if (data.Length < 3) continue;

                var itemInfo = Envir.Main.GetItemInfo(data[0]);
                if (itemInfo == null) continue;

                if (ushort.TryParse(data[2], out ushort dura))
                {
                    ingredientDuraByIndex[itemInfo.Index] = dura;
                }
            }
        }

        var name = Path.GetFileNameWithoutExtension(fileWithExt);
        var info = new RecipeInfo(name);
        if (info.Item == null)
        {
            throw new Exception($"failed to build recipe for {name} (missing product item?)");
        }

        var fixture = new RecipeFixture
        {
            recipe_file = NormalizePath(fileWithExt),
            product = FromRecipeProduct(info.Item),
            chance = info.Chance,
            gold = info.Gold,
            tools = info.Tools.Select(FromRecipeTool).ToList(),
            ingredients = info.Ingredients.Select(i => FromRecipeIngredient(i, ingredientDuraByIndex)).ToList(),
            required_flag = info.RequiredFlag.ToList(),
            required_level = info.RequiredLevel,
            required_quest = info.RequiredQuest.ToList(),
            required_class = info.RequiredClass.Select(c => (int)c).ToList(),
            required_gender = info.RequiredGender == null ? null : (int?)info.RequiredGender.Value,
        };

        var json = JsonSerializer.Serialize(fixture, new JsonSerializerOptions { WriteIndented = true });
        File.WriteAllText(outPath, json);
        return 0;
    }

    private static RecipeItemNode FromRecipeProduct(UserItem item)
    {
        return new RecipeItemNode
        {
            item_index = item.ItemIndex,
            item_name = item.Info?.Name,
            count = item.Count,
            current_dura = 0,
            max_dura = 0,
        };
    }

    private static RecipeItemNode FromRecipeTool(UserItem item)
    {
        return new RecipeItemNode
        {
            item_index = item.ItemIndex,
            item_name = item.Info?.Name,
            count = 1,
            current_dura = 0,
            max_dura = 0,
        };
    }

    private static RecipeItemNode FromRecipeIngredient(UserItem item, Dictionary<int, ushort> duraByIndex)
    {
        ushort dura = 0;
        if (duraByIndex.TryGetValue(item.ItemIndex, out ushort v))
        {
            dura = v;
        }

        return new RecipeItemNode
        {
            item_index = item.ItemIndex,
            item_name = item.Info?.Name,
            count = item.Count,
            current_dura = dura,
            max_dura = 0,
        };
    }

    private class RecipeFixture
    {
        public string recipe_file { get; set; }
        public RecipeItemNode product { get; set; }
        public byte chance { get; set; }
        public uint gold { get; set; }
        public List<RecipeItemNode> tools { get; set; }
        public List<RecipeItemNode> ingredients { get; set; }
        public List<int> required_flag { get; set; }
        public ushort? required_level { get; set; }
        public List<int> required_quest { get; set; }
        public List<int> required_class { get; set; }
        public int? required_gender { get; set; }
    }

    private class RecipeItemNode
    {
        public int item_index { get; set; }
        public string item_name { get; set; }
        public ushort count { get; set; }
        public ushort current_dura { get; set; }
        public ushort max_dura { get; set; }
    }

    private static int RunExportMapInfos(string[] args)
    {
        string root = null;
        string outPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--out") outPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        Directory.SetCurrentDirectory(root);

        if (!Envir.Main.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Main.LoadDB()");
        }

        using (var stream = File.Create(outPath))
        using (var writer = new BinaryWriter(stream))
        {
            writer.Write(Envir.Main.MapInfoList.Count);
            for (int i = 0; i < Envir.Main.MapInfoList.Count; i++)
            {
                Envir.Main.MapInfoList[i].Save(writer);
            }
        }

        return 0;
    }

    private static int RunExportItemInfos(string[] args)
    {
        string root = null;
        string outPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--out") outPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        Directory.SetCurrentDirectory(root);

        if (!Envir.Main.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Main.LoadDB()");
        }

        using (var stream = File.Create(outPath))
        using (var writer = new BinaryWriter(stream))
        {
            writer.Write(Envir.Main.ItemInfoList.Count);
            for (int i = 0; i < Envir.Main.ItemInfoList.Count; i++)
            {
                Envir.Main.ItemInfoList[i].Save(writer);
            }
        }

        return 0;
    }

    private static int RunExportMonsterInfos(string[] args)
    {
        string root = null;
        string outPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--out") outPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        Directory.SetCurrentDirectory(root);

        if (!Envir.Main.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Main.LoadDB()");
        }

        using (var stream = File.Create(outPath))
        using (var writer = new BinaryWriter(stream))
        {
            writer.Write(Envir.Main.MonsterInfoList.Count);
            for (int i = 0; i < Envir.Main.MonsterInfoList.Count; i++)
            {
                Envir.Main.MonsterInfoList[i].Save(writer);
            }
        }

        return 0;
    }

    private static int RunExportNpcInfos(string[] args)
    {
        string root = null;
        string outPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--out") outPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        Directory.SetCurrentDirectory(root);

        if (!Envir.Main.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Main.LoadDB()");
        }

        using (var stream = File.Create(outPath))
        using (var writer = new BinaryWriter(stream))
        {
            writer.Write(Envir.Main.NPCInfoList.Count);
            for (int i = 0; i < Envir.Main.NPCInfoList.Count; i++)
            {
                Envir.Main.NPCInfoList[i].Save(writer);
            }
        }

        return 0;
    }

    private static int RunExportQuestInfos(string[] args)
    {
        string root = null;
        string outPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--out") outPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        Directory.SetCurrentDirectory(root);

        if (!Envir.Main.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Main.LoadDB()");
        }

        using (var stream = File.Create(outPath))
        using (var writer = new BinaryWriter(stream))
        {
            writer.Write(Envir.Main.QuestInfoList.Count);
            for (int i = 0; i < Envir.Main.QuestInfoList.Count; i++)
            {
                Envir.Main.QuestInfoList[i].Save(writer);
            }
        }

        return 0;
    }

    private static int RunExportMagicInfos(string[] args)
    {
        string root = null;
        string outPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--out") outPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root)) throw new Exception("--root is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        Directory.SetCurrentDirectory(root);

        if (!Envir.Main.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Main.LoadDB()");
        }

        using (var stream = File.Create(outPath))
        using (var writer = new BinaryWriter(stream))
        {
            writer.Write(Envir.Main.MagicInfoList.Count);
            for (int i = 0; i < Envir.Main.MagicInfoList.Count; i++)
            {
                Envir.Main.MagicInfoList[i].Save(writer);
            }
        }

        return 0;
    }

    private static int RunExportGameShopItems(string[] args)
    {
        string root = null;
        string outPath = null;
        string mirdbPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--out") outPath = args[++i];
            else if (a == "--mirdb") mirdbPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root) && string.IsNullOrWhiteSpace(mirdbPath))
            throw new Exception("--root or --mirdb is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        if (string.IsNullOrWhiteSpace(mirdbPath))
        {
            mirdbPath = Path.Combine(root, "Server.MirDB");
        }

        if (!File.Exists(mirdbPath))
            throw new Exception($"Server.MirDB not found: {mirdbPath}");

        var dbDir = Path.GetDirectoryName(mirdbPath);
        if (string.IsNullOrWhiteSpace(dbDir))
            throw new Exception($"Invalid Server.MirDB path: {mirdbPath}");

        // Envir.DatabasePath is relative (./Server.MirDB). Make sure Envir.Edit.LoadDB()
        // reads the intended DB by setting CWD.
        Directory.SetCurrentDirectory(dbDir);

        if (!Envir.Edit.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Edit.LoadDB()");
        }

        var dir = Path.GetDirectoryName(outPath);
        if (!string.IsNullOrWhiteSpace(dir))
        {
            Directory.CreateDirectory(dir);
        }

        using (var stream = File.Create(outPath))
        using (var writer = new BinaryWriter(stream))
        {
            writer.Write(Envir.Edit.GameShopList.Count);
            for (int i = 0; i < Envir.Edit.GameShopList.Count; i++)
                Envir.Edit.GameShopList[i].Save(writer, packet: false);
        }

        return 0;
    }

    private static int RunExportConquestInfos(string[] args)
    {
        string root = null;
        string outPath = null;
        string mirdbPath = null;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a == "--root") root = args[++i];
            else if (a == "--out") outPath = args[++i];
            else if (a == "--mirdb") mirdbPath = args[++i];
            else throw new Exception($"unknown arg: {a}");
        }

        if (string.IsNullOrWhiteSpace(root) && string.IsNullOrWhiteSpace(mirdbPath))
            throw new Exception("--root or --mirdb is required");
        if (string.IsNullOrWhiteSpace(outPath)) throw new Exception("--out is required");

        if (string.IsNullOrWhiteSpace(mirdbPath))
        {
            mirdbPath = Path.Combine(root, "Server.MirDB");
        }

        if (!File.Exists(mirdbPath))
            throw new Exception($"Server.MirDB not found: {mirdbPath}");

        var dbDir = Path.GetDirectoryName(mirdbPath);
        if (string.IsNullOrWhiteSpace(dbDir))
            throw new Exception($"Invalid Server.MirDB path: {mirdbPath}");

        Directory.SetCurrentDirectory(dbDir);

        if (!Envir.Edit.LoadDB())
        {
            throw new Exception("failed to load Server.MirDB via Envir.Edit.LoadDB()");
        }

        var dir = Path.GetDirectoryName(outPath);
        if (!string.IsNullOrWhiteSpace(dir))
        {
            Directory.CreateDirectory(dir);
        }

        using (var stream = File.Create(outPath))
        using (var writer = new BinaryWriter(stream))
        {
            writer.Write(Envir.Edit.ConquestInfoList.Count);
            for (int i = 0; i < Envir.Edit.ConquestInfoList.Count; i++)
            {
                Envir.Edit.ConquestInfoList[i].Save(writer);
            }
        }

        return 0;
    }
}

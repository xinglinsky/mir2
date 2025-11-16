using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;

namespace Server.MirDatabase
{
    internal static class QuestDetailTranslations
    {
        private static bool _initialized;

        // Key: (QuestIndex, Type, LineIndex)
        private static readonly Dictionary<Tuple<int, string, int>, string> Lines =
            new Dictionary<Tuple<int, string, int>, string>();

        private static void Initialize()
        {
            if (_initialized) return;
            _initialized = true;

            try
            {
                // 服务器运行目录：Build/Server/<Config>
                string baseDir = AppDomain.CurrentDomain.BaseDirectory;

                // 回到解决方案根目录 mir2（../../..）
                string solutionRoot = Path.GetFullPath(Path.Combine(baseDir, "..", "..", ".."));

                string csvPath = Path.Combine(solutionRoot, "docs", "translations", "quest_details_translation.csv");
                if (!File.Exists(csvPath)) return;

                string[] allLines = File.ReadAllLines(csvPath, Encoding.UTF8);
                if (allLines.Length <= 1) return;

                for (int i = 1; i < allLines.Length; i++)
                {
                    string line = allLines[i];
                    if (string.IsNullOrWhiteSpace(line)) continue;

                    string[] parts = line.Split(';');
                    if (parts.Length < 7) continue;

                    int questIndex;
                    if (!int.TryParse(parts[0], out questIndex)) continue;

                    string type = parts[3].Trim();

                    int lineIndex;
                    if (!int.TryParse(parts[4], out lineIndex)) lineIndex = 0;

                    string chinese = parts[6];
                    if (string.IsNullOrWhiteSpace(chinese)) continue;

                    var key = Tuple.Create(questIndex, type, lineIndex);
                    Lines[key] = chinese;
                }
            }
            catch
            {
                // 任何异常都不影响服务器运行，只是忽略任务文本翻译
            }
        }

        public static void Apply(QuestInfo quest)
        {
            if (quest == null) return;

            Initialize();
            if (Lines.Count == 0) return;

            ReplaceList(quest.Index, "Description", quest.Description);
            ReplaceList(quest.Index, "TaskDescription", quest.TaskDescription);
            ReplaceList(quest.Index, "ReturnDescription", quest.ReturnDescription);
            ReplaceList(quest.Index, "CompletionDescription", quest.CompletionDescription);

            quest.GotoMessage = ReplaceSingle(quest.Index, "GotoMessage", quest.GotoMessage);
            quest.KillMessage = ReplaceSingle(quest.Index, "KillMessage", quest.KillMessage);
            quest.ItemMessage = ReplaceSingle(quest.Index, "ItemMessage", quest.ItemMessage);
            quest.FlagMessage = ReplaceSingle(quest.Index, "FlagMessage", quest.FlagMessage);
        }

        private static void ReplaceList(int questIndex, string type, List<string> list)
        {
            if (list == null || list.Count == 0) return;

            for (int i = 0; i < list.Count; i++)
            {
                var key = Tuple.Create(questIndex, type, i);
                string value;
                if (Lines.TryGetValue(key, out value) && !string.IsNullOrWhiteSpace(value))
                {
                    list[i] = value;
                }
            }
        }

        private static string ReplaceSingle(int questIndex, string type, string current)
        {
            if (string.IsNullOrEmpty(current)) return current;

            var key = Tuple.Create(questIndex, type, 0);
            string value;
            if (Lines.TryGetValue(key, out value) && !string.IsNullOrWhiteSpace(value))
                return value;

            return current;
        }
    }
}

using Client.MirControls;
using Client.MirGraphics;
using Client.MirSounds;

namespace Client.MirScenes.Dialogs
{
    public sealed class HelpDialog : MirImageControl
    {
        public List<HelpPage> Pages = new List<HelpPage>();

        public MirButton CloseButton, NextButton, PreviousButton;
        public MirLabel PageLabel;
        public HelpPage CurrentPage;

        public int CurrentPageNumber = 0;

        public HelpDialog()
        {
            Index = 920;
            Library = Libraries.Prguse;
            Movable = true;
            Sort = true;

            Location = Center;

            MirImageControl TitleLabel = new MirImageControl
            {
                Index = 57,
                Library = Libraries.Title,
                Location = new Point(18, 9),
                Parent = this
            };

            PreviousButton = new MirButton
            {
                Index = 240,
                HoverIndex = 241,
                PressedIndex = 242,
                Library = Libraries.Prguse2,
                Parent = this,
                Size = new Size(16, 16),
                Location = new Point(210, 485),
                Sound = SoundList.ButtonA,
            };
            PreviousButton.Click += (o, e) =>
            {
                CurrentPageNumber--;

                if (CurrentPageNumber < 0) CurrentPageNumber = Pages.Count - 1;

                DisplayPage(CurrentPageNumber);
            };

            NextButton = new MirButton
            {
                Index = 243,
                HoverIndex = 244,
                PressedIndex = 245,
                Library = Libraries.Prguse2,
                Parent = this,
                Size = new Size(16, 16),
                Location = new Point(310, 485),
                Sound = SoundList.ButtonA,
            };
            NextButton.Click += (o, e) =>
            {
                CurrentPageNumber++;

                if (CurrentPageNumber > Pages.Count - 1) CurrentPageNumber = 0;

                DisplayPage(CurrentPageNumber);
            };

            PageLabel = new MirLabel
            {
                Text = "",
                Font = new Font(Settings.FontName, 9F),
                DrawFormat = TextFormatFlags.VerticalCenter | TextFormatFlags.HorizontalCenter,
                Parent = this,
                NotControl = true,
                Location = new Point(230, 480),
                Size = new Size(80, 20)
            };

            CloseButton = new MirButton
            {
                HoverIndex = 361,
                Index = 360,
                Location = new Point(509, 3),
                Library = Libraries.Prguse2,
                Parent = this,
                PressedIndex = 362,
                Sound = SoundList.ButtonA,
            };
            CloseButton.Click += (o, e) => Hide();

            LoadImagePages();

            DisplayPage();
        }

        private void LoadImagePages()
        {
            Point location = new Point(12, 35);

            Dictionary<string, string> keybinds = new Dictionary<string, string>();

            List<HelpPage> imagePages = new List<HelpPage> { 
                new HelpPage("快捷键信息", -1, new ShortcutPage1 { Parent = this } ) { Parent = this, Location = location, Visible = false }, 
                new HelpPage("快捷键信息", -1, new ShortcutPage2 { Parent = this } ) { Parent = this, Location = location, Visible = false }, 
                new HelpPage("聊天快捷键", -1, new ShortcutPage3 { Parent = this } ) { Parent = this, Location = location, Visible = false }, 
                new HelpPage("移动", 0, null) { Parent = this, Location = location, Visible = false }, 
                new HelpPage("攻击", 1, null) { Parent = this, Location = location, Visible = false }, 
                new HelpPage("拾取物品", 2, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("生命值", 3, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("技能", 4, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("技能", 5, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("法力值", 6, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("聊天", 7, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("队伍", 8, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("耐久度", 9, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("购买", 10, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("出售", 11, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("修理", 12, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("交易", 13, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("查看", 14, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("属性", 15, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("属性", 16, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("属性", 17, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("属性", 18, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("属性", 19, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("属性", 20, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("任务", 21, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("任务", 22, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("任务", 23, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("任务", 24, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("坐骑", 25, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("坐骑", 26, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("钓鱼", 27, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("宝石与宝珠", 28, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("英雄", 29, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("英雄", 30, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("英雄", 31, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("英雄", 32, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("英雄", 33, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("行会增益", 34, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("行会增益", 35, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("行会增益", 36, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("觉醒", 37, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("觉醒", 38, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("觉醒", 39, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("觉醒", 40, null) { Parent = this, Location = location, Visible = false },
                new HelpPage("觉醒", 41, null) { Parent = this, Location = location, Visible = false },
            };

            Pages.AddRange(imagePages);
        }


        public void DisplayPage(string pageName)
        {
            if (Pages.Count < 1) return;

            for (int i = 0; i < Pages.Count; i++)
            {
                if (Pages[i].Title.ToLower() != pageName.ToLower()) continue;

                DisplayPage(i);
                break;
            }
        }

        public void DisplayPage(int id = 0)
        {
            if (Pages.Count < 1) return;

            if (id > Pages.Count - 1) id = Pages.Count - 1;
            if (id < 0) id = 0;

            if (CurrentPage != null)
            {
                CurrentPage.Visible = false;
                if (CurrentPage.Page != null) CurrentPage.Page.Visible = false;
            }

            CurrentPage = Pages[id];

            if (CurrentPage == null) return;

            CurrentPage.Visible = true;
            if (CurrentPage.Page != null) CurrentPage.Page.Visible = true;
            CurrentPageNumber = id;

            CurrentPage.PageTitleLabel.Text = id + 1 + ". " + CurrentPage.Title;

            PageLabel.Text = string.Format("{0} / {1}", id + 1, Pages.Count);

            Show();
        }


        public void Toggle()
        {
            if (!Visible)
                Show();
            else
                Hide();
        }
    }

    public class ShortcutPage1 : ShortcutInfoPage
    {
        public ShortcutPage1()
        {
            Shortcuts = new List<ShortcutInfo>
            {
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Exit), "退出游戏"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Logout), "返回登录界面"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Bar1Skill1) + "-" + CMain.InputKeys.GetKey(KeybindOptions.Bar1Skill8), "技能快捷键"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Inventory), "背包窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Equipment), "状态窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Skills), "技能窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Group), "队伍窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Trade), "交易窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Friends), "好友窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Minimap), "小地图（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Guilds), "行会窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.GameShop), "商城窗口（打开/关闭）"),
                //Shortcuts.Add(new ShortcutInfo("K", "Rental window (open / close)"));
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Relationship), "关系窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Belt), "腰带窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Options), "系统设置窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Help), "帮助窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Mount), "上马/下马"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.TargetSpellLockOn), "技能锁定目标而非鼠标位置")
            };

            LoadKeyBinds();
        }
    }
    public class ShortcutPage2 : ShortcutInfoPage
    {
        public ShortcutPage2()
        {
            Shortcuts = new List<ShortcutInfo>
            {
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.ChangePetmode), "切换宠物攻击模式"),
                //Shortcuts.Add(new ShortcutInfo("Ctrl + F", "Change the font in the chat box"));
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.ChangeAttackmode), "切换玩家攻击模式"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.AttackmodePeace), "和平模式：只攻击怪物"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.AttackmodeGroup), "组队模式：除队友外可攻击所有目标"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.AttackmodeGuild), "行会模式：除行会成员外可攻击所有目标"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.AttackmodeRedbrown), "善恶模式：只攻击PK玩家和怪物"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.AttackmodeAll), "全体模式：可攻击所有目标"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Bigmap), "显示大地图"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Skillbar), "显示技能栏"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Autorun), "自动奔跑 开/关"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Cameramode), "显示/隐藏界面"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Pickup), "高亮/拾取物品"),
                new ShortcutInfo("Ctrl + Right Click", "查看其他玩家装备"),
                //Shortcuts.Add(new ShortcutInfo("F12", "Chat macros"));
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Screenshot), "截图"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Fishing), "打开/关闭钓鱼窗口"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.Mentor), "师徒窗口（打开/关闭）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.CreaturePickup), "宠物拾取（多目标）"),
                new ShortcutInfo(CMain.InputKeys.GetKey(KeybindOptions.CreatureAutoPickup), "宠物拾取（单目标）")
            };

            LoadKeyBinds();
        }
    }
    public class ShortcutPage3 : ShortcutInfoPage
    {
        public ShortcutPage3()
        {
            Shortcuts = new List<ShortcutInfo>
            {
                //Shortcuts.Add(new ShortcutInfo("` / Ctrl", "Change the skill bar"));
                new ShortcutInfo("/(username)", "向指定玩家发送密语"),
                new ShortcutInfo("!(text)", "向附近玩家喊话"),
                new ShortcutInfo("!~(text)", "在行会频道发言")
            };

            LoadKeyBinds();
        }
    }

    public class ShortcutInfo
    {
        public string Shortcut { get; set; }
        public string Information { get; set; }

        public ShortcutInfo(string shortcut, string info)
        {
            Shortcut = shortcut.Replace("\n", " + ");
            Information = info;
        }
    }

    public class ShortcutInfoPage : MirControl
    {
        protected List<ShortcutInfo> Shortcuts = new List<ShortcutInfo>();

        public ShortcutInfoPage()
        {
            Visible = false;

            MirLabel shortcutTitleLabel = new MirLabel
            {
                Text = "快捷键",
                DrawFormat = TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter,
                ForeColour = Color.White,
                Font = new Font(Settings.FontName, 10F),
                Parent = this,
                AutoSize = true,
                Location = new Point(13, 75),
                Size = new Size(100, 30)
            };

            MirLabel infoTitleLabel = new MirLabel
            {
                Text = "说明",
                DrawFormat = TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter,
                ForeColour = Color.White,
                Font = new Font(Settings.FontName, 10F),
                Parent = this,
                AutoSize = true,
                Location = new Point(114, 75),
                Size = new Size(405, 30)
            };
        }

        public void LoadKeyBinds()
        {
            if (Shortcuts == null) return;

            for (int i = 0; i < Shortcuts.Count; i++)
            {
                MirLabel shortcutLabel = new MirLabel
                {
                    Text = Shortcuts[i].Shortcut,
                    ForeColour = Color.Yellow,
                    DrawFormat = TextFormatFlags.VerticalCenter,
                    Font = new Font(Settings.FontName, 9F),
                    Parent = this,
                    AutoSize = true,
                    Location = new Point(18, 107 + (20 * i)),
                    Size = new Size(95, 23),
                };

                MirLabel informationLabel = new MirLabel
                {
                    Text = Shortcuts[i].Information,
                    DrawFormat = TextFormatFlags.VerticalCenter,
                    ForeColour = Color.White,
                    Font = new Font(Settings.FontName, 9F),
                    Parent = this,
                    AutoSize = true,
                    Location = new Point(119, 107 + (20 * i)),
                    Size = new Size(400, 23),
                };
            }  
        }
    }

    public class HelpPage : MirControl
    {
        public string Title;
        public int ImageID;
        public MirControl Page;

        public MirLabel PageTitleLabel;

        public HelpPage(string title, int imageID, MirControl page)
        {
            Title = title;
            ImageID = imageID;
            Page = page;

            NotControl = true;
            Size = new System.Drawing.Size(508, 396 + 40);

            BeforeDraw += HelpPage_BeforeDraw;

            PageTitleLabel = new MirLabel
            {
                Text = Title,
                Font = new Font(Settings.FontName, 10F, FontStyle.Bold),
                DrawFormat = TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter,
                Parent = this,
                Size = new System.Drawing.Size(242, 30),
                Location = new Point(135, 4)
            };
        }

        void HelpPage_BeforeDraw(object sender, EventArgs e)
        {
            if (ImageID < 0) return;

            Libraries.Help.Draw(ImageID, new Point(DisplayLocation.X, DisplayLocation.Y + 40), Color.White, false);
        }
    }
}

public class GameLanguage
{
    //Client
    public static string PetMode_Both = "[Pet: Attack and Move]",
                         PetMode_MoveOnly = "[Pet: Do Not Attack]",
                         PetMode_AttackOnly = "[Pet: Do Not Move]",
                         PetMode_None = "[Pet: Do Not Attack or Move]",
                         PetMode_FocusMasterTarget = "[Pet: Focus Master Target]",

                         AttackMode_Peace = "[Mode: Peaceful]",
                         AttackMode_Group = "[Mode: Group]",
                         AttackMode_Guild = "[Mode: Guild]",
                         AttackMode_EnemyGuild = "[Mode: Enemy Guild]",
                         AttackMode_RedBrown = "[Mode: Red/Brown]",
                         AttackMode_All = "[Mode: Attack All]",

                         LogOutTip = "Do you want to log out of Legend of Mir?",
                         ExitTip = "Do you want to quit Legend of Mir?",
                         DiedTip = "You have died, Do you want to revive in town?",
                         DropTip = "Are you sure you want to drop {0}?",

                         Inventory = "Inventory ({0})",
                         Character = "Character ({0})",
                         Skills = "Skills ({0})",
                         Quests = "Quests ({0})",
                         Options = "Options ({0})",
                         Menu = "Menu",
                         GameShop = "Game Shop ({0})",
                         BigMap = "BigMap ({0})",
                         DuraPanel = "Dura Panel",
                         Mail = "Mail",
                         Exit = "Exit ({0})",
                         LogOut = "Log Out ({0})",
                         Help = "Help ({0})",
                         Keybinds = "Keybinds",
                         Ranking = "Ranking ({0})",
                         Creatures = "Creatures ({0})",
                         Mount = "Mount ({0})",
                         Fishing = "Fishing ({0})",
                         Friends = "Friends ({0})",
                         Mentor = "Mentor ({0})",
                         Relationship = "Relationship ({0})",
                         Groups = "Groups ({0})",
                         Guild = "Guild ({0})",
                         Expire = "Expire: {0}",
                         ExpireNever = "Expire: Never",
                         ExpirePaused = "Expire: Paused",
                         Never = "Never",
                         Trade = "Trade ({0})",
                         Size = "Size",
                         ChatSettings = "Chat Settings",
                         Rotate = "Rotate",
                         Close = "Close ({0})",
                         GameMaster = "GameMaster",

                         PatchErr = "Could not get Patch Information",
                         LastOnline = "Last Online",

                         Gold = "Gold",
                         Credit = "Credit",
                         GameShop_BuyWithGold = "Buy with Gold",
                         GameShop_BuyWithCredits = "Buy with Credits",
                         GameShop_BuyWithGoldHint = "Buy item(s) with Gold.",
                         GameShop_BuyWithCreditsHint = "Buy item(s) with Credits.",
                         GameShop_ShowAll = "Show All",
                         GameShop_PageText = "{0} / {1}",

                         YouGained = "You gained {0}.",

                         YouGained2 = "You gained {0:###,###,###} {1}",

                         ExperienceGained = "Experience Gained {0}",

                         HeroInventory = "Hero Inventory ({0})",
                         HeroCharacter = "Hero Character ({0})",
                         HeroSkills = "Hero Skills ({0})",
                         HeroExperienceGained = "Hero Experience Gained {0}",

                         ItemDescription = "Item Description",
                         RequiredLevel = "Required Level : {0}",
                         RequiredDC = "Required DC : {0}",
                         RequiredMC = "Required MC : {0}",
                         RequiredSC = "Required SC : {0}",
                         ClassRequired = "Class Required : {0}",

                         Holy = "Holy: + {0} (+{1})",
                         Holy2 = "Holy: + {0}",
                         Accuracy = "Accuracy: + {0} (+{1})",
                         Accuracy2 = "Accuracy: + {0}",
                         Agility = "Agility: + {0} (+{1})",
                         Agility2 = "Agility: + {0}",
                         DC = "DC + {0}~{1} (+{2})",
                         DC2 = "DC + {0}~{1}",
                         MC = "MC + {0}~{1} (+{2})",
                         MC2 = "MC + {0}~{1}",
                         SC = "SC + {0}~{1} (+{2})",
                         SC2 = "SC + {0}~{1}",
                         Durability = "Durability",
                         Weight = "W:",
                         AC = "AC + {0}~{1} (+{2})",
                         AC2 = "AC + {0}~{1}",
                         MAC = "MAC + {0}~{1} (+{2})",
                         MAC2 = "MAC + {0}~{1}",
                         Luck = "Luck + {0}",

                         DeleteCharacter = "Are you sure you want to Delete the character {0}",
                         CharacterDeleted = "Your character was deleted successfully.",
                         CharacterCreated = "Your character was created successfully.",

                         Resolution = "Resolution",
                         Autostart = "Auto start",
                         Usrname = "Username",
                         Password = "Password",

                         ShuttingDown = "Disconnected: Server is shutting down.",
                         MaxCombine = "Max Combine Count : {0}{1}Shift + Left click to split the stack",
                         Count = " Count {0}",
                         ExtraSlots8 = "Are you sure you would like to buy 8 extra slots for 1,000,000 gold?" +
                         "Next purchase you can unlock 4 extra slots up to a maximum of 40 slots.",
                         ExtraSlots4 = "Are you sure you would like to unlock 4 extra slots? for gold: {0:###,###}",

                         Chat_All = "All",
                         Chat_Short = "Shout",
                         Chat_Whisper = "Whisper",
                         Chat_Lover = "Lover",
                         Chat_Mentor = "Mentor",
                         Chat_Group = "Group",
                         Chat_Guild = "Guild",
                         ExpandedStorageLocked = "Expanded Storage Locked",
                         ExtraStorage = "Would you like to rent extra storage for 10 days at a cost of 1,000,000 gold?",
                         ExtendYourRentalPeriod = "Would you like to extend your rental period for 10 days at a cost of 1,000,000 gold?",

                         CannotLeaveGame = "Cannot leave game for {0} seconds",
                         SelectKey = "Select the Key for: {0}",

                         WeaponSpiritFire = "Your weapon is glowed by spirit of fire.",
                         SpiritsFireDisappeared = "The spirits of fire disappeared.",
                         WeddingRing = "WeddingRing",
                         ItemTextFormat = "{0}{1}{2} {3}",
                         DropAmount = "Drop Amount:",
                         LowMana = "Not Enough Mana to cast.",
                         NoCreatures = "You do not own any creatures.",
                         NoMount = "You do not own a mount.",
                         NoFishingRod = "You are not holding a fishing rod.",
                         AttemptingConnect = "Attempting to connect to the server.{0}Attempt:{1}",

                         CreatingCharactersDisabled = "Creating new characters is currently disabled.",
                         InvalidCharacterName = "Your Character Name is not acceptable.",
                         NoClass = "The class you selected does not exist. Contact a GM for assistance.",
                         ToManyCharacters = "You cannot make anymore then {0} Characters.",
                         CharacterNameExists = "A Character with this name already exists.",

                         Client_WrongVersion = "Wrong version, please update your game.\nGame will now Close",

                         Account_CreationDisabled = "Account creation is currently disabled.",
                         Account_IDNotAcceptable = "Your AccountID is not acceptable.",
                         Account_PasswordNotAcceptable = "Your Password is not acceptable.",
                         Account_EmailNotAcceptable = "Your E-Mail Address is not acceptable.",
                         Account_UserNameNotAcceptable = "Your User Name is not acceptable.",
                         Account_SecretQuestionNotAcceptable = "Your Secret Question is not acceptable.",
                         Account_SecretAnswerNotAcceptable = "Your Secret Answer is not acceptable.",
                         Account_IDAlreadyExists = "An Account with this ID already exists.",
                         Account_Created = "Your account was created successfully.",

                         PasswordChange_Disabled = "Password Changing is currently disabled.",
                         PasswordChange_CurrentNotAcceptable = "The current Password is not acceptable.",
                         PasswordChange_NewNotAcceptable = "Your new Password is not acceptable.",
                         PasswordChange_Success = "Your password was changed successfully.",

                         Login_Disabled = "Logging in is currently disabled.",
                         Login_PasswordChangeRequired = "The account's password must be changed before logging in.",

                         Account_Banned = "This account is banned.\n\nReason: {0}\nExpiryDate: {1}\nDuration: {2:#,##0} Hours, {3} Minutes, {4} Seconds",

                         Character_GenderNotExist = "The gender you selected does not exist.\n Contact a GM for assistance.",
                         Character_EnterName = "Please enter the characters name.",
                         Character_IncorrectEntry = "Incorrect Entry.",
                         DeleteCharactersDisabled = "Deleting characters is currently disabled.",
                         Character_NotExist = "The character you selected does not exist.\n Contact a GM for assistance.",
                         Character_LoginDelay = "You cannot log onto this character for another {0} seconds.",

                         StartGame_Disabled = "Starting the game is currently disabled.",
                         StartGame_NotLoggedIn = "You are not logged in.",
                         StartGame_CharacterNotFound = "Your character could not be found.",
                         StartGame_NoMapOrStartPoint = "No active map and/or start point found.",
                         WarriorsDes = "Warriors are a class of great strength and vitality. They are not easily killed in battle and have the advantage of being able to use" +
                                        " a variety of heavy weapons and Armour. Therefore, Warriors favor attacks that are based on melee physical damage. They are weak in ranged" +
                                        " attacks, however the variety of equipment that are developed specifically for Warriors complement their weakness in ranged combat.",
                         WizardDes = "Wizards are a class of low strength and stamina, but have the ability to use powerful spells. Their offensive spells are very effective, but" +
                                        " because it takes time to cast these spells, they're likely to leave themselves open for enemy's attacks. Therefore, the physically weak wizards" +
                                        " must aim to attack their enemies from a safe distance.",
                         TaoistDes = "Taoists are well disciplined in the study of Astronomy, Medicine, and others aside from Mu-Gong. Rather then directly engaging the enemies, their" +
                                        " specialty lies in assisting their allies with support. Taoists can summon powerful creatures and have a high resistance to magic, and is a class" +
                                        " with well balanced offensive and defensive abilities.",
                         AssassinDes = "Assassins are members of a secret organization and their history is relatively unknown. They're capable of hiding themselves and performing attacks" +
                                        " while being unseen by others, which naturally makes them excellent at making fast kills. It is necessary for them to avoid being in battles with" +
                                        " multiple enemies due to their weak vitality and strength.",
                         ArcherDes = "Archers are a class of great accuracy and strength, using their powerful skills with bows to deal extraordinary damage from range. Much like" +
                                        " wizards, they rely on their keen instincts to dodge oncoming attacks as they tend to leave themselves open to frontal attacks. However, their" +
                                        " physical prowess and deadly aim allows them to instil fear into anyone they hit.",
                         DateSent = "Date Sent : {0}",
                         Send = "Send",
                         Reply = "Reply",
                         Read = "Read",
                         Delete = "Delete",
                         BlockList = "Block List",
                         EnterMailToName = "Please enter the name of the person you would like to mail.",
                         AddFriend = "Add",
                         RemoveFriend = "Remove",
                         FriendMemo = "Memo",
                         FriendMail = "Mail",
                         FriendWhisper = "Whisper",
                         FriendEnterAddName = "Please enter the name of the person you would like to Add.",
                         FriendEnterBlockName = "Please enter the name of the person you would like to Block.",
                         Friend_RemoveConfirm = "Are you sure you wish to remove '{0}'?",
                         System_PlayerNotOnline = "Player is not online",
                         AddMentor = "Add Mentor",
                         RemoveMentorMentee = "Remove Mentor/Mentee",
                         MentorRequests = "Allow/Disallow Mentor Requests",
                         MentorEnterName = "Please enter the name of the person you would like to be your Mentor.",
                         Mentor_Online = "ONLINE",
                         Mentor_MentorLabel = "MENTOR",
                         Mentor_MenteeLabel = "MENTEE",
                         Mentor_MenteeExp = "MENTEE EXP: {0}",
                         Mentor_LevelPrefix = "Lv ",
                         Mentor_AlreadyHasMentor = "You already have a Mentor.",
                         Mentor_CancelWarning = "Cancelling a Mentorship early will cause a cooldown. Are you sure?",
                         RestedBuff = "Rested{0}Increases Exp Rate by {1}%{2}",

                         SkillMode_Tilde = "[Skill Mode: ~]",
                         SkillMode_Ctrl = "[Skill Mode: Ctrl]",
                         MainDialog_HP = "HP {0}/{1}",
                         MainDialog_MP = "MP {0}/{1} ",

                         Option_HPMPMode1 = "[HP/MP Mode 1]",
                         Option_HPMPMode2 = "[HP/MP Mode 2]",
                         Option_NewMove = "[New Movement Style]",
                         Option_OldMove = "[Old Movement Style]",

                         Keyboard_Layout = "Keyboard ({0})",

                         Chat_Report = "Report",

                         Ranking_OnlineOnly = "Online Only",

                         Keyboard_EnforceStrict = "Assign Rule: Strict",
                         Keyboard_EnforceRelaxed = "Assign Rule: Relaxed",

                         InputKey_Esc = "Esc",
                         InputKey_Delete = "Delete",
                         InputKey_Enter = "Enter",
                         InputKey_Random = "Random",

                         Guild_ShowOffline = "Show Offline",
                         Guild_StatusHeaders = "Guild Name\n\nLevel\n\nMembers",
                         Guild_RecruitMember = "Recruit Member",
                         Guild_Buff_InsufficientLevel = "Insufficient Level",
                         Guild_Buff_Available = "Available",
                         Guild_Buff_CountingDown = "Counting down.",
                         Guild_Buff_Expired = "Expired.",
                         Guild_Buff_Obtained = "Obtained.",
                         Guild_Buff_Active = "Active",
                         Guild_Buff_Inactive = "Inactive",
                         Guild_Buff_MinLevel = "Minimum Guild Level: {0}",
                         Guild_Buff_PointsRequired = "Points Required: {0}",
                         Guild_Buff_ActivationCost = "Activation Cost: {0} gold.",
                         Guild_EditRank = "Edit Rank",
                         Guild_SelectRank = "Select Rank",
                         Guild_RankNoBuffPermission = "Guild rank does not allow buff activation.",
                         Guild_ChangeRankConfirm = "Are you sure you want to change the rank of {0} to {1}?",
                         Guild_KickMemberConfirm = "Are you sure you want to kick {0}?",
                         Guild_CreateRankConfirm = "Are you sure you want to create a new rank?",

                         ItemTypeWeapon = "Weapon",
                         ItemTypeArmour = "Armour",
                         ItemTypeHelmet = "Helmet",
                         ItemTypeNecklace = "Necklace",
                         ItemTypeBracelet = "Bracelet",
                         ItemTypeRing = "Ring",
                         ItemTypeAmulet = "Amulet",
                         ItemTypeBelt = "Belt",
                         ItemTypeBoots = "Boots",
                         ItemTypeStone = "Stone",
                         ItemTypeTorch = "Torch",
                         ItemTypePotion = "Potion",
                         ItemTypeOre = "Ore",
                         ItemTypeMeat = "Meat",
                         ItemTypeCraftingMaterial = "CraftingMaterial",
                         ItemTypeScroll = "Scroll",
                         ItemTypeGem = "Gem",
                         ItemTypeMount = "Mount",
                         ItemTypeBook = "Book",
                         ItemTypeScript = "Script",
                         ItemTypeReins = "Reins",
                         ItemTypeBells = "Bells",
                         ItemTypeSaddle = "Saddle",
                         ItemTypeRibbon = "Ribbon",
                         ItemTypeMask = "Mask",
                         ItemTypeFood = "Food",
                         ItemTypeHook = "Hook",
                         ItemTypeFloat = "Float",
                         ItemTypeBait = "Bait",
                         ItemTypeFinder = "Finder",
                         ItemTypeReel = "Reel",
                         ItemTypeFish = "Fish",
                         ItemTypeQuest = "Quest",
                         ItemTypeAwakening = "Awakening",
                         ItemTypePets = "Pets",
                         ItemTypeTransform = "Transform",
                         ItemTypeDeco = "Deco",
                         ItemTypeMonsterSpawn = "SpawnEgg",
                         ItemTypeSealedHero = "SealedHero",

                         ItemGradeCommon = "Common",
                         ItemGradeRare = "Rare",
                         ItemGradeLegendary = "Legendary",
                         ItemGradeMythical = "Mythical",
                         ItemGradeHeroic = "Heroic",
                         NoAccountID = "The AccountID does not exist.",
                         IncorrectPasswordAccountID = "Incorrect Password and AccountID combination.",
                         GroupSwitch = "Allow/Disallow Group Requests",
                         GroupAdd = "Add",
                         GroupRemove = "Remove",
                         GroupAddEnterName = "Please enter the name of the person you wish to add.",
                         GroupRemoveEnterName = "Please enter the name of the person you wish to remove.",
                         Group_MaxMembers = "Your group already has the maximum number of members.",
                         Group_NotLeader = "You are not the leader of your group.",
                         TooHeavyToHold = "It is too heavy to Hold.",
                         SwitchMarriage = "Allow/Block Marriage",
                         RequestMarriage = "Request Marriage",
                         RequestDivorce = "Request Divorce",
                         MailLover = "Mail Lover",
                         WhisperLover = "Whisper Lover",
                         Relationship_LoverLabel = "Lover:  {0}",
                         Relationship_Location = "Location:  {0}",
                         Relationship_LocationOffline = "Location:  Offline",
                         Relationship_Date = "Date: ",
                         Relationship_Length = "Length: ",
                         Relationship_DivorcedDate = "Divorced Date:  {0}",
                         Relationship_TimeSinceDays = "Time Since: {0} Days",
                         Relationship_MarriageDate = "Marriage Date:  {0}",
                         Relationship_LengthDays = "Length: {0} Days",
                         Relationship_AlreadyMarried = "You're already married.",
                         Relationship_NotMarried = "You're not married.",
                         Relationship_LoverNotOnline = "Lover is not online",
                         Relationship_AllowBlockRecall = "Allow/Block Recall",

                         // Skill related
                         Skill_NoSuitableWeapon = "You must be wearing a suitable weapon to perform this skill",
                         Skill_CannotCast = "You cannot cast {0} for another {1} seconds.",
                         Skill_UseThrusting = "Use Thrusting.",
                         Skill_DoNotUseThrusting = "Do not use Thrusting.",
                         Skill_UseHalfMoon = "Use Half Moon.",
                         Skill_DoNotUseHalfMoon = "Do not use Half Moon.",
                         Skill_UseCrossHalfMoon = "Use Cross Half Moon.",
                         Skill_DoNotUseCrossHalfMoon = "Do not use Cross Half Moon.",
                         Skill_UseDoubleSlash = "Use Double Slash.",
                         Skill_DoNotUseDoubleSlash = "Do not use Double Slash.",

                         // System messages
                         System_NothingFound = "Nothing Found.",
                         System_PleaseEnterInfo = "Please enter the required information.",
                         System_PersonObservingLoggedOff = "The person you was observing has logged off.",
                         System_UnknownTypeRequired = "Unknown Type Required",

                         // Group related
                         Group_YouLeft = "You have left the group.",
                         Group_PlayerLeft = "{0} has left the group.",
                         Group_PlayerJoined = "{0} has joined the group.",
                         Group_InviteQuestion = "Do you want to group with {0}?",

                         // Quest related
                         Quest_ShareQuestion = "{0} would like to share a quest with you. Do you accept?",

                         // Item related
                         Item_LocationAt = "{0} at {1}",
                         Item_NoLongerLoyal = "{0} is no longer loyal to you.",
                         Item_DuraDroppedToZero = "{0}'s dura has dropped to 0.",
                         Item_AddsDurability = "Adds +{0} Durability",
                         Item_AddsAccuracy = "Adds +{0} Accuracy",
                         Item_AddsASpeed = "Adds +{0} A.Speed",
                         Item_AddsFreezing = "Adds +{0} Freezing",
                         Item_AddsPoison = "Adds +{0} Poison",
                         Item_AddsAgility = "Adds +{0} Agility",
                         Item_AddsPoisonResist = "Adds +{0} Poison Resist",
                         Item_AddsMagicResist = "Adds +{0} Magic Resist",
                         Item_InstantRun = "Instant Run",
                         Item_Socket = "Socket : {0}",
                         Item_SocketEmpty = "Empty",
                         Item_SocketOpenHint = "Ctrl + Right Click To Open Sockets",
                         Item_SellingPrice = "Selling Price : {0} Gold",
                         Item_CantDropOnDeath = "Can't drop on death",
                         Item_CantDrop = "Can't drop",
                         Item_CantUpgrade = "Can't upgrade",
                         Item_CantSell = "Can't sell",
                         Item_CantTrade = "Can't trade",
                         Item_CantStore = "Can't store",
                         Item_CantRepair = "Can't repair",

                         // Damage types
                         Damage_Miss = "Miss",
                         Damage_Crit = "Crit",

                         // Error messages
                         Error_CouldNotGetDisplayResolutions = "Could not get display resolutions",
                         Error_GetDisplayResolutionIssue = "Get Display Resolution Issue",
                         Error_InvalidClientResolution = "Invalid Client Resolution";

    //Server
    public static string Welcome = "Welcome to the {0} Server.",
                         OnlinePlayers = "Online Players: {0}",
                         WeaponLuck = "Luck dwells within your weapon.",
                         WeaponCurse = "Curse dwells within your weapon.",
                         WeaponNoEffect = "No effect.",
                         InventoryIncreased = "Inventory size increased.",
                         FaceToTrade = "You must face someone to trade.",
                         NoTownTeleport = "You cannot use Town Teleports here",
                         CanNotRandom = "You cannot use Random Teleports here",
                         CanNotDungeon = "You cannot use Dungeon Escapes here",
                         CannotResurrection = "You cannot use Resurrection Scrolls whilst alive",
                         CanNotDrop = "You cannot drop items on this map",
                         NewMail = "New mail has arrived.",
                         CouldNotFindPlayer = "Could not find player {0}",
                         BeenPoisoned = "You have been poisoned",
                         AllowingMentorRequests = "You're now allowing mentor requests.",
                         BlockingMentorRequests = "You're now blocking mentor requests.";

    //common
    public static string LowLevel = "You are not a high enough level.",
                         LowGold = "Not enough gold.",
                         LevelUp = "Congratulations! You have leveled up. Your HP and MP have been restored.",
                         LowDC = "You do not have enough DC.",
                         LowMC = "You do not have enough MC.",
                         LowSC = "You do not have enough SC.",
                         GameName = "Legend of Mir 2",
                         ExpandedStorageExpiresOn = "Expanded Storage Expires On",

                         NotFemale = "You are not Female.",
                         NotMale = "You are not Male.",
                         NotInGuild = "You are not in a guild.",
                         NoMentorship = "You don't currently have a Mentorship to cancel.",
                         NoBagSpace = "You do not have enough space.";


    public static void LoadClientLanguage(string languageIniPath)
    {
        if (!File.Exists(languageIniPath))
        {
            SaveClientLanguage(languageIniPath);
            return;
        }

        InIReader reader = new InIReader(languageIniPath);
        GameLanguage.PetMode_Both = reader.ReadString("Language", "PetMode_Both", GameLanguage.PetMode_Both);
        GameLanguage.PetMode_MoveOnly = reader.ReadString("Language", "PetMode_MoveOnly", GameLanguage.PetMode_MoveOnly);
        GameLanguage.PetMode_AttackOnly = reader.ReadString("Language", "PetMode_AttackOnly", GameLanguage.PetMode_AttackOnly);
        GameLanguage.PetMode_None = reader.ReadString("Language", "PetMode_None", GameLanguage.PetMode_None);
        GameLanguage.PetMode_FocusMasterTarget = reader.ReadString("Language", "PetMode_FocusMasterTarget", GameLanguage.PetMode_FocusMasterTarget);

        GameLanguage.AttackMode_Peace = reader.ReadString("Language", "AttackMode_Peace", GameLanguage.AttackMode_Peace);
        GameLanguage.AttackMode_Group = reader.ReadString("Language", "AttackMode_Group", GameLanguage.AttackMode_Group);
        GameLanguage.AttackMode_Guild = reader.ReadString("Language", "AttackMode_Guild", GameLanguage.AttackMode_Guild);
        GameLanguage.AttackMode_EnemyGuild = reader.ReadString("Language", "AttackMode_EnemyGuild", GameLanguage.AttackMode_EnemyGuild);
        GameLanguage.AttackMode_RedBrown = reader.ReadString("Language", "AttackMode_RedBrown", GameLanguage.AttackMode_RedBrown);
        GameLanguage.AttackMode_All = reader.ReadString("Language", "AttackMode_All", GameLanguage.AttackMode_All);

        GameLanguage.LogOutTip = reader.ReadString("Language", "LogOutTip", GameLanguage.LogOutTip);
        GameLanguage.ExitTip = reader.ReadString("Language", "ExitTip", GameLanguage.ExitTip);
        GameLanguage.DiedTip = reader.ReadString("Language", "DiedTip", GameLanguage.DiedTip);
        GameLanguage.DropTip = reader.ReadString("Language", "DropTip", GameLanguage.DropTip);

        GameLanguage.Inventory = reader.ReadString("Language", "Inventory", GameLanguage.Inventory);
        GameLanguage.Character = reader.ReadString("Language", "Character", GameLanguage.Character);
        GameLanguage.Skills = reader.ReadString("Language", "Skills", GameLanguage.Skills);
        GameLanguage.Quests = reader.ReadString("Language", "Quests", GameLanguage.Quests);
        GameLanguage.Options = reader.ReadString("Language", "Options", GameLanguage.Options);
        GameLanguage.Menu = reader.ReadString("Language", "Menu", GameLanguage.Menu);
        GameLanguage.GameShop = reader.ReadString("Language", "GameShop", GameLanguage.GameShop);
        GameLanguage.BigMap = reader.ReadString("Language", "BigMap", GameLanguage.BigMap);
        GameLanguage.DuraPanel = reader.ReadString("Language", "DuraPanel", GameLanguage.DuraPanel);
        GameLanguage.Mail = reader.ReadString("Language", "Mail", GameLanguage.Mail);
        GameLanguage.Exit = reader.ReadString("Language", "Exit", GameLanguage.Exit);
        GameLanguage.LogOut = reader.ReadString("Language", "LogOut", GameLanguage.LogOut);
        GameLanguage.Help = reader.ReadString("Language", "Help", GameLanguage.Help);
        GameLanguage.Keybinds = reader.ReadString("Language", "Keybinds", GameLanguage.Keybinds);
        GameLanguage.Ranking = reader.ReadString("Language", "Ranking", GameLanguage.Ranking);
        GameLanguage.Creatures = reader.ReadString("Language", "Creatures", GameLanguage.Creatures);
        GameLanguage.Mount = reader.ReadString("Language", "Mount", GameLanguage.Mount);
        GameLanguage.Fishing = reader.ReadString("Language", "Fishing", GameLanguage.Fishing);
        GameLanguage.Friends = reader.ReadString("Language", "Friends", GameLanguage.Friends);
        GameLanguage.Mentor = reader.ReadString("Language", "Mentor", GameLanguage.Mentor);
        GameLanguage.Relationship = reader.ReadString("Language", "Relationship", GameLanguage.Relationship);
        GameLanguage.Groups = reader.ReadString("Language", "Groups", GameLanguage.Groups);
        GameLanguage.Guild = reader.ReadString("Language", "Guild", GameLanguage.Guild);
        GameLanguage.Trade = reader.ReadString("Language", "Trade", GameLanguage.Trade);
        GameLanguage.Size = reader.ReadString("Language", "Size", GameLanguage.Size);
        GameLanguage.ChatSettings = reader.ReadString("Language", "ChatSettings", GameLanguage.ChatSettings);
        GameLanguage.Rotate = reader.ReadString("Language", "Rotate", GameLanguage.Rotate);
        GameLanguage.Close = reader.ReadString("Language", "Close", GameLanguage.Close);
        GameLanguage.GameMaster = reader.ReadString("Language", "GameMaster", GameLanguage.GameMaster);
        GameLanguage.Expire = reader.ReadString("Language", "Expire", GameLanguage.Expire);
        GameLanguage.ExpireNever = reader.ReadString("Language", "ExpireNever", GameLanguage.ExpireNever);
        GameLanguage.ExpirePaused = reader.ReadString("Language", "ExpirePaused", GameLanguage.ExpirePaused);
        GameLanguage.Never = reader.ReadString("Language", "Never", GameLanguage.Never);

        GameLanguage.PatchErr = reader.ReadString("Language", "PatchErr", GameLanguage.PatchErr);
        GameLanguage.LastOnline = reader.ReadString("Language", "LastOnline", GameLanguage.LastOnline);

        GameLanguage.LowLevel = reader.ReadString("Language", "LowLevel", GameLanguage.LowLevel);
        GameLanguage.LowGold = reader.ReadString("Language", "LowGold", GameLanguage.LowGold);
        GameLanguage.LowDC = reader.ReadString("Language", "LowDC", GameLanguage.LowDC);
        GameLanguage.LowMC = reader.ReadString("Language", "LowMC", GameLanguage.LowMC);
        GameLanguage.LowSC = reader.ReadString("Language", "LowSC", GameLanguage.LowSC);

        GameLanguage.Gold = reader.ReadString("Language", "Gold", GameLanguage.Gold);
        GameLanguage.Credit = reader.ReadString("Language", "Credit", GameLanguage.Credit);

        GameLanguage.GameShop_BuyWithGold = reader.ReadString("Language", "GameShop_BuyWithGold", GameLanguage.GameShop_BuyWithGold);
        GameLanguage.GameShop_BuyWithCredits = reader.ReadString("Language", "GameShop_BuyWithCredits", GameLanguage.GameShop_BuyWithCredits);
        GameLanguage.GameShop_BuyWithGoldHint = reader.ReadString("Language", "GameShop_BuyWithGoldHint", GameLanguage.GameShop_BuyWithGoldHint);
        GameLanguage.GameShop_BuyWithCreditsHint = reader.ReadString("Language", "GameShop_BuyWithCreditsHint", GameLanguage.GameShop_BuyWithCreditsHint);
        GameLanguage.GameShop_ShowAll = reader.ReadString("Language", "GameShop_ShowAll", GameLanguage.GameShop_ShowAll);
        GameLanguage.GameShop_PageText = reader.ReadString("Language", "GameShop_PageText", GameLanguage.GameShop_PageText);

        GameLanguage.YouGained = reader.ReadString("Language", "YouGained", GameLanguage.YouGained);
        GameLanguage.YouGained2 = reader.ReadString("Language", "YouGained2", GameLanguage.YouGained2);
        GameLanguage.ExperienceGained = reader.ReadString("Language", "ExperienceGained", GameLanguage.ExperienceGained);        
        GameLanguage.LevelUp = reader.ReadString("Language", "LevelUp", GameLanguage.LevelUp);

        GameLanguage.HeroInventory = reader.ReadString("Language", "HeroInventory", GameLanguage.HeroInventory);
        GameLanguage.HeroCharacter = reader.ReadString("Language", "HeroCharacter", GameLanguage.HeroCharacter);
        GameLanguage.HeroSkills = reader.ReadString("Language", "HeroSkills", GameLanguage.HeroSkills);
        GameLanguage.HeroExperienceGained = reader.ReadString("Language", "HeroExperienceGained", GameLanguage.HeroExperienceGained);

        GameLanguage.ItemDescription = reader.ReadString("Language", "ItemDescription", GameLanguage.ItemDescription);
        GameLanguage.RequiredLevel = reader.ReadString("Language", "RequiredLevel", GameLanguage.RequiredLevel);
        GameLanguage.RequiredDC = reader.ReadString("Language", "RequiredDC", GameLanguage.RequiredDC);
        GameLanguage.RequiredMC = reader.ReadString("Language", "RequiredMC", GameLanguage.RequiredMC);
        GameLanguage.RequiredSC = reader.ReadString("Language", "RequiredSC", GameLanguage.RequiredSC);
        GameLanguage.ClassRequired = reader.ReadString("Language", "ClassRequired", GameLanguage.ClassRequired);
        GameLanguage.Holy = reader.ReadString("Language", "Holy", GameLanguage.Holy);
        GameLanguage.Holy2 = reader.ReadString("Language", "Holy2", GameLanguage.Holy2);
        GameLanguage.Accuracy = reader.ReadString("Language", "Accuracy", GameLanguage.Accuracy);
        GameLanguage.Accuracy2 = reader.ReadString("Language", "Accuracy2", GameLanguage.Accuracy2);
        GameLanguage.Agility = reader.ReadString("Language", "Agility", GameLanguage.Agility);
        GameLanguage.Agility2 = reader.ReadString("Language", "Agility2", GameLanguage.Agility2);
        GameLanguage.DC = reader.ReadString("Language", "DC", GameLanguage.DC);
        GameLanguage.DC2 = reader.ReadString("Language", "DC2", GameLanguage.DC2);
        GameLanguage.MC = reader.ReadString("Language", "MC", GameLanguage.MC);
        GameLanguage.MC2 = reader.ReadString("Language", "MC2", GameLanguage.MC2);
        GameLanguage.SC = reader.ReadString("Language", "SC", GameLanguage.SC);
        GameLanguage.SC2 = reader.ReadString("Language", "SC2", GameLanguage.SC2);
        GameLanguage.Durability = reader.ReadString("Language", "Durability", GameLanguage.Durability);
        GameLanguage.Weight = reader.ReadString("Language", "Weight", GameLanguage.Weight);
        GameLanguage.AC = reader.ReadString("Language", "AC", GameLanguage.AC);
        GameLanguage.AC2 = reader.ReadString("Language", "AC2", GameLanguage.AC2);
        GameLanguage.MAC = reader.ReadString("Language", "MAC", GameLanguage.MAC);
        GameLanguage.MAC2 = reader.ReadString("Language", "MAC2", GameLanguage.MAC2);
        GameLanguage.Luck = reader.ReadString("Language", "Luck", GameLanguage.Luck);

        GameLanguage.DeleteCharacter = reader.ReadString("Language", "DeleteCharacter", GameLanguage.DeleteCharacter);
        GameLanguage.CharacterDeleted = reader.ReadString("Language", "CharacterDeleted", GameLanguage.CharacterDeleted);
        GameLanguage.CharacterCreated = reader.ReadString("Language", "CharacterCreated", GameLanguage.CharacterCreated);

        GameLanguage.Resolution = reader.ReadString("Language", "Resolution", GameLanguage.Resolution);
        GameLanguage.Autostart = reader.ReadString("Language", "Autostart", GameLanguage.Autostart);
        GameLanguage.Usrname = reader.ReadString("Language", "Usrname", GameLanguage.Usrname);
        GameLanguage.Password = reader.ReadString("Language", "Password", GameLanguage.Password);

        GameLanguage.ShuttingDown = reader.ReadString("Language", "ShuttingDown", GameLanguage.ShuttingDown);

        GameLanguage.MaxCombine = reader.ReadString("Language", "MaxCombine", GameLanguage.MaxCombine);
        GameLanguage.Count = reader.ReadString("Language", "Count", GameLanguage.Count);
        GameLanguage.ExtraSlots8 = reader.ReadString("Language", "ExtraSlots8", GameLanguage.ExtraSlots8);
        GameLanguage.ExtraSlots4 = reader.ReadString("Language", "ExtraSlots4", GameLanguage.ExtraSlots4);

        GameLanguage.Chat_All = reader.ReadString("Language", "Chat_All", GameLanguage.Chat_All);
        GameLanguage.Chat_Short = reader.ReadString("Language", "Chat_Short", GameLanguage.Chat_Short);
        GameLanguage.Chat_Whisper = reader.ReadString("Language", "Chat_Whisper", GameLanguage.Chat_Whisper);
        GameLanguage.Chat_Lover = reader.ReadString("Language", "Chat_Lover", GameLanguage.Chat_Lover);
        GameLanguage.Chat_Mentor = reader.ReadString("Language", "Chat_Mentor", GameLanguage.Chat_Mentor);
        GameLanguage.Chat_Group = reader.ReadString("Language", "Chat_Group", GameLanguage.Chat_Group);
        GameLanguage.Chat_Guild = reader.ReadString("Language", "Chat_Guild", GameLanguage.Chat_Guild);
        GameLanguage.ExpandedStorageLocked = reader.ReadString("Language", "ExpandedStorageLocked", GameLanguage.ExpandedStorageLocked);
        GameLanguage.ExtraStorage = reader.ReadString("Language", "ExtraStorage", GameLanguage.ExtraStorage);
        GameLanguage.ExtendYourRentalPeriod = reader.ReadString("Language", "ExtendYourRentalPeriod", GameLanguage.ExtendYourRentalPeriod);
        GameLanguage.ExpandedStorageExpiresOn = reader.ReadString("Language", "ExpandedStorageExpiresOn", GameLanguage.ExpandedStorageExpiresOn);
        GameLanguage.GameName = reader.ReadString("Language", "GameName", GameLanguage.GameName);
        GameLanguage.CannotLeaveGame = reader.ReadString("Language", "CannotLeaveGame", GameLanguage.CannotLeaveGame);
        GameLanguage.SelectKey = reader.ReadString("Language", "SelectKey", GameLanguage.SelectKey);
        GameLanguage.WeaponSpiritFire = reader.ReadString("Language", "WeaponSpiritFire", GameLanguage.WeaponSpiritFire);
        GameLanguage.SpiritsFireDisappeared = reader.ReadString("Language", "SpiritsFireDisappeared", GameLanguage.SpiritsFireDisappeared);
        GameLanguage.WeddingRing = reader.ReadString("Language", "WeddingRing", GameLanguage.WeddingRing);
        GameLanguage.ItemTextFormat = reader.ReadString("Language", "ItemTextFormat", GameLanguage.ItemTextFormat);
        GameLanguage.DropAmount = reader.ReadString("Language", "DropAmount", GameLanguage.DropAmount);
        GameLanguage.LowMana = reader.ReadString("Language", "LowMana", GameLanguage.LowMana);

        GameLanguage.NotFemale = reader.ReadString("Language", "NotFemale", GameLanguage.NotFemale);
        GameLanguage.NotMale = reader.ReadString("Language", "NotMale", GameLanguage.NotMale);
        GameLanguage.NoCreatures = reader.ReadString("Language", "NoCreatures", GameLanguage.NoCreatures);
        GameLanguage.NoMount = reader.ReadString("Language", "NoMount", GameLanguage.NoMount);
        GameLanguage.NoFishingRod = reader.ReadString("Language", "NoFishingRod", GameLanguage.NoFishingRod);
        GameLanguage.NotInGuild = reader.ReadString("Language", "NotInGuild", GameLanguage.NotInGuild);
        GameLanguage.NoBagSpace = reader.ReadString("Language", "NoBagSpace", GameLanguage.NoBagSpace);
        GameLanguage.AttemptingConnect = reader.ReadString("Language", "AttemptingConnect", GameLanguage.AttemptingConnect);

        GameLanguage.CreatingCharactersDisabled = reader.ReadString("Language", "CreatingCharactersDisabled", GameLanguage.CreatingCharactersDisabled);
        GameLanguage.InvalidCharacterName = reader.ReadString("Language", "InvalidCharacterName", GameLanguage.InvalidCharacterName);
        GameLanguage.NoClass = reader.ReadString("Language", "NoClass", GameLanguage.NoClass);
        GameLanguage.ToManyCharacters = reader.ReadString("Language", "ToManyCharacters", GameLanguage.ToManyCharacters);
        GameLanguage.CharacterNameExists = reader.ReadString("Language", "CharacterNameExists", GameLanguage.CharacterNameExists);

        GameLanguage.Client_WrongVersion = reader.ReadString("Language", "Client_WrongVersion", GameLanguage.Client_WrongVersion);

        GameLanguage.Account_CreationDisabled = reader.ReadString("Language", "Account_CreationDisabled", GameLanguage.Account_CreationDisabled);
        GameLanguage.Account_IDNotAcceptable = reader.ReadString("Language", "Account_IDNotAcceptable", GameLanguage.Account_IDNotAcceptable);
        GameLanguage.Account_PasswordNotAcceptable = reader.ReadString("Language", "Account_PasswordNotAcceptable", GameLanguage.Account_PasswordNotAcceptable);
        GameLanguage.Account_EmailNotAcceptable = reader.ReadString("Language", "Account_EmailNotAcceptable", GameLanguage.Account_EmailNotAcceptable);
        GameLanguage.Account_UserNameNotAcceptable = reader.ReadString("Language", "Account_UserNameNotAcceptable", GameLanguage.Account_UserNameNotAcceptable);
        GameLanguage.Account_SecretQuestionNotAcceptable = reader.ReadString("Language", "Account_SecretQuestionNotAcceptable", GameLanguage.Account_SecretQuestionNotAcceptable);
        GameLanguage.Account_SecretAnswerNotAcceptable = reader.ReadString("Language", "Account_SecretAnswerNotAcceptable", GameLanguage.Account_SecretAnswerNotAcceptable);
        GameLanguage.Account_IDAlreadyExists = reader.ReadString("Language", "Account_IDAlreadyExists", GameLanguage.Account_IDAlreadyExists);
        GameLanguage.Account_Created = reader.ReadString("Language", "Account_Created", GameLanguage.Account_Created);

        GameLanguage.PasswordChange_Disabled = reader.ReadString("Language", "PasswordChange_Disabled", GameLanguage.PasswordChange_Disabled);
        GameLanguage.PasswordChange_CurrentNotAcceptable = reader.ReadString("Language", "PasswordChange_CurrentNotAcceptable", GameLanguage.PasswordChange_CurrentNotAcceptable);
        GameLanguage.PasswordChange_NewNotAcceptable = reader.ReadString("Language", "PasswordChange_NewNotAcceptable", GameLanguage.PasswordChange_NewNotAcceptable);
        GameLanguage.PasswordChange_Success = reader.ReadString("Language", "PasswordChange_Success", GameLanguage.PasswordChange_Success);

        GameLanguage.Login_Disabled = reader.ReadString("Language", "Login_Disabled", GameLanguage.Login_Disabled);
        GameLanguage.Login_PasswordChangeRequired = reader.ReadString("Language", "Login_PasswordChangeRequired", GameLanguage.Login_PasswordChangeRequired);

        GameLanguage.Account_Banned = reader.ReadString("Language", "Account_Banned", GameLanguage.Account_Banned);

        GameLanguage.Character_GenderNotExist = reader.ReadString("Language", "Character_GenderNotExist", GameLanguage.Character_GenderNotExist);
        GameLanguage.Character_EnterName = reader.ReadString("Language", "Character_EnterName", GameLanguage.Character_EnterName);
        GameLanguage.Character_IncorrectEntry = reader.ReadString("Language", "Character_IncorrectEntry", GameLanguage.Character_IncorrectEntry);
        GameLanguage.DeleteCharactersDisabled = reader.ReadString("Language", "DeleteCharactersDisabled", GameLanguage.DeleteCharactersDisabled);
        GameLanguage.Character_NotExist = reader.ReadString("Language", "Character_NotExist", GameLanguage.Character_NotExist);
        GameLanguage.Character_LoginDelay = reader.ReadString("Language", "Character_LoginDelay", GameLanguage.Character_LoginDelay);

        GameLanguage.StartGame_Disabled = reader.ReadString("Language", "StartGame_Disabled", GameLanguage.StartGame_Disabled);
        GameLanguage.StartGame_NotLoggedIn = reader.ReadString("Language", "StartGame_NotLoggedIn", GameLanguage.StartGame_NotLoggedIn);
        GameLanguage.StartGame_CharacterNotFound = reader.ReadString("Language", "StartGame_CharacterNotFound", GameLanguage.StartGame_CharacterNotFound);
        GameLanguage.StartGame_NoMapOrStartPoint = reader.ReadString("Language", "StartGame_NoMapOrStartPoint", GameLanguage.StartGame_NoMapOrStartPoint);

        GameLanguage.WarriorsDes = reader.ReadString("Language", "WarriorsDes", GameLanguage.WarriorsDes);
        GameLanguage.WizardDes = reader.ReadString("Language", "WizardDes", GameLanguage.WizardDes);
        GameLanguage.TaoistDes = reader.ReadString("Language", "TaoistDes", GameLanguage.TaoistDes);
        GameLanguage.AssassinDes = reader.ReadString("Language", "AssassinDes", GameLanguage.AssassinDes);
        GameLanguage.ArcherDes = reader.ReadString("Language", "ArcherDes", GameLanguage.ArcherDes);

        GameLanguage.DateSent = reader.ReadString("Language", "DateSent", GameLanguage.DateSent);
        GameLanguage.Send = reader.ReadString("Language", "Send", GameLanguage.Send);
        GameLanguage.Reply = reader.ReadString("Language", "Reply", GameLanguage.Reply);
        GameLanguage.Read = reader.ReadString("Language", "Read", GameLanguage.Read);
        GameLanguage.Delete = reader.ReadString("Language", "Delete", GameLanguage.Delete);
        GameLanguage.BlockList = reader.ReadString("Language", "BlockList", GameLanguage.BlockList);
        GameLanguage.EnterMailToName = reader.ReadString("Language", "EnterMailToName", GameLanguage.EnterMailToName);
        GameLanguage.BeenPoisoned = reader.ReadString("Language", "BeenPoisoned", GameLanguage.BeenPoisoned);
        GameLanguage.AddFriend = reader.ReadString("Language", "AddFriend", GameLanguage.AddFriend);
        GameLanguage.RemoveFriend = reader.ReadString("Language", "RemoveFriend", GameLanguage.RemoveFriend);
        GameLanguage.FriendMemo = reader.ReadString("Language", "FriendMemo", GameLanguage.FriendMemo);
        GameLanguage.FriendMail = reader.ReadString("Language", "FriendMail", GameLanguage.FriendMail);
        GameLanguage.FriendWhisper = reader.ReadString("Language", "FriendWhisper", GameLanguage.FriendWhisper);
        GameLanguage.FriendEnterAddName = reader.ReadString("Language", "FriendEnterAddName", GameLanguage.FriendEnterAddName);
        GameLanguage.FriendEnterBlockName = reader.ReadString("Language", "FriendEnterBlockName", GameLanguage.FriendEnterBlockName);
        GameLanguage.AddMentor = reader.ReadString("Language", "AddMentor", GameLanguage.AddMentor);
        GameLanguage.RemoveMentorMentee = reader.ReadString("Language", "RemoveMentorMentee", GameLanguage.RemoveMentorMentee);
        GameLanguage.MentorRequests = reader.ReadString("Language", "MentorRequests", GameLanguage.MentorRequests);
        GameLanguage.MentorEnterName = reader.ReadString("Language", "MentorEnterName", GameLanguage.MentorEnterName);
        GameLanguage.NoMentorship = reader.ReadString("Language", "NoMentorship", GameLanguage.NoMentorship);
        GameLanguage.RestedBuff = reader.ReadString("Language", "RestedBuff", GameLanguage.RestedBuff);

        GameLanguage.SkillMode_Tilde = reader.ReadString("Language", "SkillMode_Tilde", GameLanguage.SkillMode_Tilde);
        GameLanguage.SkillMode_Ctrl = reader.ReadString("Language", "SkillMode_Ctrl", GameLanguage.SkillMode_Ctrl);
        GameLanguage.MainDialog_HP = reader.ReadString("Language", "MainDialog_HP", GameLanguage.MainDialog_HP);
        GameLanguage.MainDialog_MP = reader.ReadString("Language", "MainDialog_MP", GameLanguage.MainDialog_MP);

        GameLanguage.Option_HPMPMode1 = reader.ReadString("Language", "Option_HPMPMode1", GameLanguage.Option_HPMPMode1);
        GameLanguage.Option_HPMPMode2 = reader.ReadString("Language", "Option_HPMPMode2", GameLanguage.Option_HPMPMode2);
        GameLanguage.Option_NewMove = reader.ReadString("Language", "Option_NewMove", GameLanguage.Option_NewMove);
        GameLanguage.Option_OldMove = reader.ReadString("Language", "Option_OldMove", GameLanguage.Option_OldMove);

        GameLanguage.Keyboard_Layout = reader.ReadString("Language", "Keyboard_Layout", GameLanguage.Keyboard_Layout);

        GameLanguage.Chat_Report = reader.ReadString("Language", "Chat_Report", GameLanguage.Chat_Report);

        GameLanguage.Ranking_OnlineOnly = reader.ReadString("Language", "Ranking_OnlineOnly", GameLanguage.Ranking_OnlineOnly);

        GameLanguage.Keyboard_EnforceStrict = reader.ReadString("Language", "Keyboard_EnforceStrict", GameLanguage.Keyboard_EnforceStrict);
        GameLanguage.Keyboard_EnforceRelaxed = reader.ReadString("Language", "Keyboard_EnforceRelaxed", GameLanguage.Keyboard_EnforceRelaxed);

        GameLanguage.InputKey_Esc = reader.ReadString("Language", "InputKey_Esc", GameLanguage.InputKey_Esc);
        GameLanguage.InputKey_Delete = reader.ReadString("Language", "InputKey_Delete", GameLanguage.InputKey_Delete);
        GameLanguage.InputKey_Enter = reader.ReadString("Language", "InputKey_Enter", GameLanguage.InputKey_Enter);
        GameLanguage.InputKey_Random = reader.ReadString("Language", "InputKey_Random", GameLanguage.InputKey_Random);

        GameLanguage.ItemTypeWeapon = reader.ReadString("Language", "ItemTypeWeapon", GameLanguage.ItemTypeWeapon);
        GameLanguage.ItemTypeArmour = reader.ReadString("Language", "ItemTypeArmour", GameLanguage.ItemTypeArmour);
        GameLanguage.ItemTypeHelmet = reader.ReadString("Language", "ItemTypeHelmet", GameLanguage.ItemTypeHelmet);
        GameLanguage.ItemTypeNecklace = reader.ReadString("Language", "ItemTypeNecklace", GameLanguage.ItemTypeNecklace);
        GameLanguage.ItemTypeBracelet = reader.ReadString("Language", "ItemTypeBracelet", GameLanguage.ItemTypeBracelet);
        GameLanguage.ItemTypeRing = reader.ReadString("Language", "ItemTypeRing", GameLanguage.ItemTypeRing);
        GameLanguage.ItemTypeAmulet = reader.ReadString("Language", "ItemTypeAmulet", GameLanguage.ItemTypeAmulet);
        GameLanguage.ItemTypeBelt = reader.ReadString("Language", "ItemTypeBelt", GameLanguage.ItemTypeBelt);
        GameLanguage.ItemTypeBoots = reader.ReadString("Language", "ItemTypeBoots", GameLanguage.ItemTypeBoots);
        GameLanguage.ItemTypeStone = reader.ReadString("Language", "ItemTypeStone", GameLanguage.ItemTypeStone);
        GameLanguage.ItemTypeTorch = reader.ReadString("Language", "ItemTypeTorch", GameLanguage.ItemTypeTorch);
        GameLanguage.ItemTypePotion = reader.ReadString("Language", "ItemTypePotion", GameLanguage.ItemTypePotion);
        GameLanguage.ItemTypeOre = reader.ReadString("Language", "ItemTypeOre", GameLanguage.ItemTypeOre);
        GameLanguage.ItemTypeMeat = reader.ReadString("Language", "ItemTypeMeat", GameLanguage.ItemTypeMeat);
        GameLanguage.ItemTypeCraftingMaterial = reader.ReadString("Language", "ItemTypeCraftingMaterial", GameLanguage.ItemTypeCraftingMaterial);
        GameLanguage.ItemTypeScroll = reader.ReadString("Language", "ItemTypeScroll", GameLanguage.ItemTypeScroll);
        GameLanguage.ItemTypeGem = reader.ReadString("Language", "ItemTypeGem", GameLanguage.ItemTypeGem);
        GameLanguage.ItemTypeMount = reader.ReadString("Language", "ItemTypeMount", GameLanguage.ItemTypeMount);
        GameLanguage.ItemTypeBook = reader.ReadString("Language", "ItemTypeBook", GameLanguage.ItemTypeBook);
        GameLanguage.ItemTypeScript = reader.ReadString("Language", "ItemTypeScript", GameLanguage.ItemTypeScript);
        GameLanguage.ItemTypeReins = reader.ReadString("Language", "ItemTypeReins", GameLanguage.ItemTypeReins);
        GameLanguage.ItemTypeBells = reader.ReadString("Language", "ItemTypeBells", GameLanguage.ItemTypeBells);
        GameLanguage.ItemTypeSaddle = reader.ReadString("Language", "ItemTypeSaddle", GameLanguage.ItemTypeSaddle);
        GameLanguage.ItemTypeRibbon = reader.ReadString("Language", "ItemTypeRibbon", GameLanguage.ItemTypeRibbon);
        GameLanguage.ItemTypeMask = reader.ReadString("Language", "ItemTypeMask", GameLanguage.ItemTypeMask);
        GameLanguage.ItemTypeFood = reader.ReadString("Language", "ItemTypeFood", GameLanguage.ItemTypeFood);
        GameLanguage.ItemTypeHook = reader.ReadString("Language", "ItemTypeHook", GameLanguage.ItemTypeHook);
        GameLanguage.ItemTypeFloat = reader.ReadString("Language", "ItemTypeFloat", GameLanguage.ItemTypeFloat);
        GameLanguage.ItemTypeBait = reader.ReadString("Language", "ItemTypeBait", GameLanguage.ItemTypeBait);
        GameLanguage.ItemTypeFinder = reader.ReadString("Language", "ItemTypeFinder", GameLanguage.ItemTypeFinder);
        GameLanguage.ItemTypeReel = reader.ReadString("Language", "ItemTypeReel", GameLanguage.ItemTypeReel);
        GameLanguage.ItemTypeFish = reader.ReadString("Language", "ItemTypeFish", GameLanguage.ItemTypeFish);
        GameLanguage.ItemTypeQuest = reader.ReadString("Language", "ItemTypeQuest", GameLanguage.ItemTypeQuest);
        GameLanguage.ItemTypeAwakening = reader.ReadString("Language", "ItemTypeAwakening", GameLanguage.ItemTypeAwakening);
        GameLanguage.ItemTypePets = reader.ReadString("Language", "ItemTypePets", GameLanguage.ItemTypePets);
        GameLanguage.ItemTypeTransform = reader.ReadString("Language", "ItemTypeTransform", GameLanguage.ItemTypeTransform);
        GameLanguage.ItemTypeSealedHero = reader.ReadString("Language", "ItemTypeSealedHero", GameLanguage.ItemTypeSealedHero);

        GameLanguage.ItemGradeCommon = reader.ReadString("Language", "ItemGradeCommon", GameLanguage.ItemGradeCommon);
        GameLanguage.ItemGradeRare = reader.ReadString("Language", "ItemGradeRare", GameLanguage.ItemGradeRare);
        GameLanguage.ItemGradeLegendary = reader.ReadString("Language", "ItemGradeLegendary", GameLanguage.ItemGradeLegendary);
        GameLanguage.ItemGradeMythical = reader.ReadString("Language", "ItemGradeMythical", GameLanguage.ItemGradeMythical);
        GameLanguage.ItemGradeHeroic = reader.ReadString("Language", "ItemGradeHeroic", GameLanguage.ItemGradeHeroic);

        GameLanguage.NoAccountID = reader.ReadString("Language", "NoAccountID", GameLanguage.NoAccountID);
        GameLanguage.IncorrectPasswordAccountID = reader.ReadString("Language", "IncorrectPasswordAccountID", GameLanguage.IncorrectPasswordAccountID);
        GameLanguage.GroupSwitch = reader.ReadString("Language", "GroupSwitch", GameLanguage.GroupSwitch);
        GameLanguage.GroupAdd = reader.ReadString("Language", "GroupAdd", GameLanguage.GroupAdd);
        GameLanguage.GroupRemove = reader.ReadString("Language", "GroupRemove", GameLanguage.GroupRemove);
        GameLanguage.GroupAddEnterName = reader.ReadString("Language", "GroupAddEnterName", GameLanguage.GroupAddEnterName);
        GameLanguage.GroupRemoveEnterName = reader.ReadString("Language", "GroupRemoveEnterName", GameLanguage.GroupRemoveEnterName);
        GameLanguage.TooHeavyToHold = reader.ReadString("Language", "TooHeavyToHold", GameLanguage.TooHeavyToHold);
        GameLanguage.SwitchMarriage = reader.ReadString("Language", "SwitchMarriage", GameLanguage.SwitchMarriage);
        GameLanguage.RequestMarriage = reader.ReadString("Language", "RequestMarriage", GameLanguage.RequestMarriage);
        GameLanguage.RequestDivorce = reader.ReadString("Language", "RequestDivorce", GameLanguage.RequestDivorce);
        GameLanguage.MailLover = reader.ReadString("Language", "MailLover", GameLanguage.MailLover);
        GameLanguage.WhisperLover = reader.ReadString("Language", "WhisperLover", GameLanguage.WhisperLover);

        // Skill related
        GameLanguage.Skill_NoSuitableWeapon = reader.ReadString("Language", "Skill_NoSuitableWeapon", GameLanguage.Skill_NoSuitableWeapon);
        GameLanguage.Skill_CannotCast = reader.ReadString("Language", "Skill_CannotCast", GameLanguage.Skill_CannotCast);
        GameLanguage.Skill_UseThrusting = reader.ReadString("Language", "Skill_UseThrusting", GameLanguage.Skill_UseThrusting);
        GameLanguage.Skill_DoNotUseThrusting = reader.ReadString("Language", "Skill_DoNotUseThrusting", GameLanguage.Skill_DoNotUseThrusting);
        GameLanguage.Skill_UseHalfMoon = reader.ReadString("Language", "Skill_UseHalfMoon", GameLanguage.Skill_UseHalfMoon);
        GameLanguage.Skill_DoNotUseHalfMoon = reader.ReadString("Language", "Skill_DoNotUseHalfMoon", GameLanguage.Skill_DoNotUseHalfMoon);
        GameLanguage.Skill_UseCrossHalfMoon = reader.ReadString("Language", "Skill_UseCrossHalfMoon", GameLanguage.Skill_UseCrossHalfMoon);
        GameLanguage.Skill_DoNotUseCrossHalfMoon = reader.ReadString("Language", "Skill_DoNotUseCrossHalfMoon", GameLanguage.Skill_DoNotUseCrossHalfMoon);
        GameLanguage.Skill_UseDoubleSlash = reader.ReadString("Language", "Skill_UseDoubleSlash", GameLanguage.Skill_UseDoubleSlash);
        GameLanguage.Skill_DoNotUseDoubleSlash = reader.ReadString("Language", "Skill_DoNotUseDoubleSlash", GameLanguage.Skill_DoNotUseDoubleSlash);

        // System messages
        GameLanguage.System_NothingFound = reader.ReadString("Language", "System_NothingFound", GameLanguage.System_NothingFound);
        GameLanguage.System_PleaseEnterInfo = reader.ReadString("Language", "System_PleaseEnterInfo", GameLanguage.System_PleaseEnterInfo);
        GameLanguage.System_PersonObservingLoggedOff = reader.ReadString("Language", "System_PersonObservingLoggedOff", GameLanguage.System_PersonObservingLoggedOff);
        GameLanguage.System_UnknownTypeRequired = reader.ReadString("Language", "System_UnknownTypeRequired", GameLanguage.System_UnknownTypeRequired);

        // Group related
        GameLanguage.Group_YouLeft = reader.ReadString("Language", "Group_YouLeft", GameLanguage.Group_YouLeft);
        GameLanguage.Group_PlayerLeft = reader.ReadString("Language", "Group_PlayerLeft", GameLanguage.Group_PlayerLeft);
        GameLanguage.Group_PlayerJoined = reader.ReadString("Language", "Group_PlayerJoined", GameLanguage.Group_PlayerJoined);
        GameLanguage.Group_InviteQuestion = reader.ReadString("Language", "Group_InviteQuestion", GameLanguage.Group_InviteQuestion);

        // Quest related
        GameLanguage.Quest_ShareQuestion = reader.ReadString("Language", "Quest_ShareQuestion", GameLanguage.Quest_ShareQuestion);

        // Item related
        GameLanguage.Item_LocationAt = reader.ReadString("Language", "Item_LocationAt", GameLanguage.Item_LocationAt);
        GameLanguage.Item_NoLongerLoyal = reader.ReadString("Language", "Item_NoLongerLoyal", GameLanguage.Item_NoLongerLoyal);
        GameLanguage.Item_DuraDroppedToZero = reader.ReadString("Language", "Item_DuraDroppedToZero", GameLanguage.Item_DuraDroppedToZero);
        GameLanguage.Item_AddsDurability = reader.ReadString("Language", "Item_AddsDurability", GameLanguage.Item_AddsDurability);
        GameLanguage.Item_AddsAccuracy = reader.ReadString("Language", "Item_AddsAccuracy", GameLanguage.Item_AddsAccuracy);
        GameLanguage.Item_AddsASpeed = reader.ReadString("Language", "Item_AddsASpeed", GameLanguage.Item_AddsASpeed);
        GameLanguage.Item_AddsFreezing = reader.ReadString("Language", "Item_AddsFreezing", GameLanguage.Item_AddsFreezing);
        GameLanguage.Item_AddsPoison = reader.ReadString("Language", "Item_AddsPoison", GameLanguage.Item_AddsPoison);
        GameLanguage.Item_AddsAgility = reader.ReadString("Language", "Item_AddsAgility", GameLanguage.Item_AddsAgility);
        GameLanguage.Item_AddsPoisonResist = reader.ReadString("Language", "Item_AddsPoisonResist", GameLanguage.Item_AddsPoisonResist);
        GameLanguage.Item_AddsMagicResist = reader.ReadString("Language", "Item_AddsMagicResist", GameLanguage.Item_AddsMagicResist);
        GameLanguage.Item_InstantRun = reader.ReadString("Language", "Item_InstantRun", GameLanguage.Item_InstantRun);
        GameLanguage.Item_Socket = reader.ReadString("Language", "Item_Socket", GameLanguage.Item_Socket);
        GameLanguage.Item_SocketEmpty = reader.ReadString("Language", "Item_SocketEmpty", GameLanguage.Item_SocketEmpty);
        GameLanguage.Item_SocketOpenHint = reader.ReadString("Language", "Item_SocketOpenHint", GameLanguage.Item_SocketOpenHint);
        GameLanguage.Item_SellingPrice = reader.ReadString("Language", "Item_SellingPrice", GameLanguage.Item_SellingPrice);
        GameLanguage.Item_CantDropOnDeath = reader.ReadString("Language", "Item_CantDropOnDeath", GameLanguage.Item_CantDropOnDeath);
        GameLanguage.Item_CantDrop = reader.ReadString("Language", "Item_CantDrop", GameLanguage.Item_CantDrop);
        GameLanguage.Item_CantUpgrade = reader.ReadString("Language", "Item_CantUpgrade", GameLanguage.Item_CantUpgrade);
        GameLanguage.Item_CantSell = reader.ReadString("Language", "Item_CantSell", GameLanguage.Item_CantSell);
        GameLanguage.Item_CantTrade = reader.ReadString("Language", "Item_CantTrade", GameLanguage.Item_CantTrade);
        GameLanguage.Item_CantStore = reader.ReadString("Language", "Item_CantStore", GameLanguage.Item_CantStore);
        GameLanguage.Item_CantRepair = reader.ReadString("Language", "Item_CantRepair", GameLanguage.Item_CantRepair);

        // Damage types
        GameLanguage.Damage_Miss = reader.ReadString("Language", "Damage_Miss", GameLanguage.Damage_Miss);
        GameLanguage.Damage_Crit = reader.ReadString("Language", "Damage_Crit", GameLanguage.Damage_Crit);

        // Error messages
        GameLanguage.Error_CouldNotGetDisplayResolutions = reader.ReadString("Language", "Error_CouldNotGetDisplayResolutions", GameLanguage.Error_CouldNotGetDisplayResolutions);
        GameLanguage.Error_GetDisplayResolutionIssue = reader.ReadString("Language", "Error_GetDisplayResolutionIssue", GameLanguage.Error_GetDisplayResolutionIssue);
        GameLanguage.Error_InvalidClientResolution = reader.ReadString("Language", "Error_InvalidClientResolution", GameLanguage.Error_InvalidClientResolution);

        LoadDatabaseTranslations(languageIniPath);
    }


    public static void SaveClientLanguage(string languageIniPath)
    {
        File.Delete(languageIniPath);
        InIReader reader = new InIReader(languageIniPath);
        reader.Write("Language", "PetMode_Both", GameLanguage.PetMode_Both);
        reader.Write("Language", "PetMode_MoveOnly", GameLanguage.PetMode_MoveOnly);
        reader.Write("Language", "PetMode_AttackOnly", GameLanguage.PetMode_AttackOnly);
        reader.Write("Language", "PetMode_None", GameLanguage.PetMode_None);
        reader.Write("Language", "PetMode_FocusMasterTarget", GameLanguage.PetMode_FocusMasterTarget);

        reader.Write("Language", "AttackMode_Peace", GameLanguage.AttackMode_Peace);
        reader.Write("Language", "AttackMode_Group", GameLanguage.AttackMode_Group);
        reader.Write("Language", "AttackMode_Guild", GameLanguage.AttackMode_Guild);
        reader.Write("Language", "AttackMode_EnemyGuild", GameLanguage.AttackMode_EnemyGuild);
        reader.Write("Language", "AttackMode_RedBrown", GameLanguage.AttackMode_RedBrown);
        reader.Write("Language", "AttackMode_All", GameLanguage.AttackMode_All);

        reader.Write("Language", "LogOutTip", GameLanguage.LogOutTip);
        reader.Write("Language", "ExitTip", GameLanguage.ExitTip);
        reader.Write("Language", "DiedTip", GameLanguage.DiedTip);
        reader.Write("Language", "DropTip", GameLanguage.DropTip);

        reader.Write("Language", "Inventory", GameLanguage.Inventory);
        reader.Write("Language", "Character", GameLanguage.Character);
        reader.Write("Language", "Skills", GameLanguage.Skills);
        reader.Write("Language", "Quests", GameLanguage.Quests);
        reader.Write("Language", "Options", GameLanguage.Options);
        reader.Write("Language", "Menu", GameLanguage.Menu);
        reader.Write("Language", "GameShop", GameLanguage.GameShop);
        reader.Write("Language", "BigMap", GameLanguage.BigMap);
        reader.Write("Language", "DuraPanel", GameLanguage.DuraPanel);
        reader.Write("Language", "Mail", GameLanguage.Mail);
        reader.Write("Language", "Exit", GameLanguage.Exit);
        reader.Write("Language", "LogOut", GameLanguage.LogOut);
        reader.Write("Language", "Help", GameLanguage.Help);
        reader.Write("Language", "Keybinds", GameLanguage.Keybinds);
        reader.Write("Language", "Ranking", GameLanguage.Ranking);
        reader.Write("Language", "Creatures", GameLanguage.Creatures);
        reader.Write("Language", "Mount", GameLanguage.Mount);
        reader.Write("Language", "Fishing", GameLanguage.Fishing);
        reader.Write("Language", "Friends", GameLanguage.Friends);
        reader.Write("Language", "Mentor", GameLanguage.Mentor);
        reader.Write("Language", "Relationship", GameLanguage.Relationship);
        reader.Write("Language", "Groups", GameLanguage.Groups);
        reader.Write("Language", "Guild", GameLanguage.Guild);
        reader.Write("Language", "Trade", GameLanguage.Trade);
        reader.Write("Language", "Size", GameLanguage.Size);
        reader.Write("Language", "ChatSettings", GameLanguage.ChatSettings);
        reader.Write("Language", "Rotate", GameLanguage.Rotate);
        reader.Write("Language", "Close", GameLanguage.Close);
        reader.Write("Language", "GameMaster", GameLanguage.GameMaster);


        reader.Write("Language", "Expire", GameLanguage.Expire);
        reader.Write("Language", "ExpireNever", GameLanguage.ExpireNever);
        reader.Write("Language", "ExpirePaused", GameLanguage.ExpirePaused);
        reader.Write("Language", "Never", GameLanguage.Never);
        reader.Write("Language", "PatchErr", GameLanguage.PatchErr);
        reader.Write("Language", "LastOnline", GameLanguage.LastOnline);

        reader.Write("Language", "LowLevel", GameLanguage.LowLevel);
        reader.Write("Language", "LowGold", GameLanguage.LowGold);
        reader.Write("Language", "LowDC", GameLanguage.LowDC);
        reader.Write("Language", "LowMC", GameLanguage.LowMC);
        reader.Write("Language", "LowSC", GameLanguage.LowSC);

        reader.Write("Language", "Gold", GameLanguage.Gold);
        reader.Write("Language", "Credit", GameLanguage.Credit);

        reader.Write("Language", "GameShop_BuyWithGold", GameLanguage.GameShop_BuyWithGold);
        reader.Write("Language", "GameShop_BuyWithCredits", GameLanguage.GameShop_BuyWithCredits);
        reader.Write("Language", "GameShop_BuyWithGoldHint", GameLanguage.GameShop_BuyWithGoldHint);
        reader.Write("Language", "GameShop_BuyWithCreditsHint", GameLanguage.GameShop_BuyWithCreditsHint);
        reader.Write("Language", "GameShop_ShowAll", GameLanguage.GameShop_ShowAll);
        reader.Write("Language", "GameShop_PageText", GameLanguage.GameShop_PageText);

        reader.Write("Language", "YouGained", GameLanguage.YouGained);
        reader.Write("Language", "YouGained2", GameLanguage.YouGained2);
        reader.Write("Language", "ExperienceGained", GameLanguage.ExperienceGained);        
        reader.Write("Language", "LevelUp", GameLanguage.LevelUp);

        reader.Write("Language", "HeroInventory", GameLanguage.Inventory);
        reader.Write("Language", "HeroCharacter", GameLanguage.Character);
        reader.Write("Language", "HeroSkills", GameLanguage.Skills);
        reader.Write("Language", "HeroExperienceGained", GameLanguage.HeroExperienceGained);

        reader.Write("Language", "ItemDescription", GameLanguage.ItemDescription);
        reader.Write("Language", "RequiredLevel", GameLanguage.RequiredLevel);
        reader.Write("Language", "RequiredDC", GameLanguage.RequiredDC);
        reader.Write("Language", "RequiredMC", GameLanguage.RequiredMC);
        reader.Write("Language", "RequiredSC", GameLanguage.RequiredSC);
        reader.Write("Language", "ClassRequired", GameLanguage.ClassRequired);
        reader.Write("Language", "Holy", GameLanguage.Holy);
        reader.Write("Language", "Accuracy", GameLanguage.Accuracy);
        reader.Write("Language", "Agility", GameLanguage.Agility);
        reader.Write("Language", "DC", GameLanguage.DC);
        reader.Write("Language", "MC", GameLanguage.MC);
        reader.Write("Language", "SC", GameLanguage.SC);
        reader.Write("Language", "Durability", GameLanguage.Durability);
        reader.Write("Language", "Weight", GameLanguage.Weight);
        reader.Write("Language", "AC", GameLanguage.AC);
        reader.Write("Language", "MAC", GameLanguage.MAC);
        reader.Write("Language", "Luck", GameLanguage.Luck);

        reader.Write("Language", "DeleteCharacter", GameLanguage.DeleteCharacter);
        reader.Write("Language", "CharacterDeleted", GameLanguage.CharacterDeleted);
        reader.Write("Language", "CharacterCreated", GameLanguage.CharacterCreated);

        reader.Write("Language", "Resolution", GameLanguage.Resolution);
        reader.Write("Language", "Autostart", GameLanguage.Autostart);
        reader.Write("Language", "Usrname", GameLanguage.Usrname);
        reader.Write("Language", "Password", GameLanguage.Password);

        reader.Write("Language", "ShuttingDown", GameLanguage.ShuttingDown);

        reader.Write("Language", "MaxCombine", GameLanguage.MaxCombine);
        reader.Write("Language", "Count", GameLanguage.Count);
        reader.Write("Language", "ExtraSlots8", GameLanguage.ExtraSlots8);
        reader.Write("Language", "ExtraSlots4", GameLanguage.ExtraSlots4);

        reader.Write("Language", "Chat_All", GameLanguage.Chat_All);
        reader.Write("Language", "Chat_Short", GameLanguage.Chat_Short);
        reader.Write("Language", "Chat_Whisper", GameLanguage.Chat_Whisper);
        reader.Write("Language", "Chat_Lover", GameLanguage.Chat_Lover);
        reader.Write("Language", "Chat_Mentor", GameLanguage.Chat_Mentor);
        reader.Write("Language", "Chat_Group", GameLanguage.Chat_Group);
        reader.Write("Language", "Chat_Guild", GameLanguage.Chat_Guild);
        reader.Write("Language", "ExpandedStorageLocked", GameLanguage.ExpandedStorageLocked);
        reader.Write("Language", "ExtraStorage", GameLanguage.ExtraStorage);
        reader.Write("Language", "ExtendYourRentalPeriod", GameLanguage.ExtendYourRentalPeriod);
        reader.Write("Language", "ExpandedStorageExpiresOn", GameLanguage.ExpandedStorageExpiresOn);
        reader.Write("Language", "GameName", GameLanguage.GameName);
        reader.Write("Language", "CannotLeaveGame", GameLanguage.CannotLeaveGame);
        reader.Write("Language", "SelectKey", GameLanguage.SelectKey);
        reader.Write("Language", "WeaponSpiritFire", GameLanguage.WeaponSpiritFire);
        reader.Write("Language", "SpiritsFireDisappeared", GameLanguage.SpiritsFireDisappeared);
        reader.Write("Language", "WeddingRing", GameLanguage.WeddingRing);
        reader.Write("Language", "ItemTextFormat", GameLanguage.ItemTextFormat);
        reader.Write("Language", "DropAmount", GameLanguage.DropAmount);
        reader.Write("Language", "LowMana", GameLanguage.LowMana);

        reader.Write("Language", "NotFemale", GameLanguage.NotFemale);
        reader.Write("Language", "NotMale", GameLanguage.NotMale);
        reader.Write("Language", "NoCreatures", GameLanguage.NoCreatures);
        reader.Write("Language", "NoMount", GameLanguage.NoMount);
        reader.Write("Language", "NoFishingRod", GameLanguage.NoFishingRod);
        reader.Write("Language", "NotInGuild", GameLanguage.NotInGuild);
        reader.Write("Language", "AttemptingConnect", GameLanguage.AttemptingConnect);
        reader.Write("Language", "NoBagSpace", GameLanguage.NoBagSpace);

        reader.Write("Language", "CreatingCharactersDisabled", GameLanguage.CreatingCharactersDisabled);
        reader.Write("Language", "InvalidCharacterName", GameLanguage.InvalidCharacterName);
        reader.Write("Language", "NoClass", GameLanguage.NoClass);
        reader.Write("Language", "ToManyCharacters", GameLanguage.ToManyCharacters);
        reader.Write("Language", "CharacterNameExists", GameLanguage.CharacterNameExists);

        reader.Write("Language", "Client_WrongVersion", GameLanguage.Client_WrongVersion);

        reader.Write("Language", "Account_CreationDisabled", GameLanguage.Account_CreationDisabled);
        reader.Write("Language", "Account_IDNotAcceptable", GameLanguage.Account_IDNotAcceptable);
        reader.Write("Language", "Account_PasswordNotAcceptable", GameLanguage.Account_PasswordNotAcceptable);
        reader.Write("Language", "Account_EmailNotAcceptable", GameLanguage.Account_EmailNotAcceptable);
        reader.Write("Language", "Account_UserNameNotAcceptable", GameLanguage.Account_UserNameNotAcceptable);
        reader.Write("Language", "Account_SecretQuestionNotAcceptable", GameLanguage.Account_SecretQuestionNotAcceptable);
        reader.Write("Language", "Account_SecretAnswerNotAcceptable", GameLanguage.Account_SecretAnswerNotAcceptable);
        reader.Write("Language", "Account_IDAlreadyExists", GameLanguage.Account_IDAlreadyExists);
        reader.Write("Language", "Account_Created", GameLanguage.Account_Created);

        reader.Write("Language", "PasswordChange_Disabled", GameLanguage.PasswordChange_Disabled);
        reader.Write("Language", "PasswordChange_CurrentNotAcceptable", GameLanguage.PasswordChange_CurrentNotAcceptable);
        reader.Write("Language", "PasswordChange_NewNotAcceptable", GameLanguage.PasswordChange_NewNotAcceptable);
        reader.Write("Language", "PasswordChange_Success", GameLanguage.PasswordChange_Success);

        reader.Write("Language", "Login_Disabled", GameLanguage.Login_Disabled);
        reader.Write("Language", "Login_PasswordChangeRequired", GameLanguage.Login_PasswordChangeRequired);

        reader.Write("Language", "Account_Banned", GameLanguage.Account_Banned);

        reader.Write("Language", "Character_GenderNotExist", GameLanguage.Character_GenderNotExist);
        reader.Write("Language", "Character_EnterName", GameLanguage.Character_EnterName);
        reader.Write("Language", "Character_IncorrectEntry", GameLanguage.Character_IncorrectEntry);
        reader.Write("Language", "DeleteCharactersDisabled", GameLanguage.DeleteCharactersDisabled);
        reader.Write("Language", "Character_NotExist", GameLanguage.Character_NotExist);
        reader.Write("Language", "Character_LoginDelay", GameLanguage.Character_LoginDelay);

        reader.Write("Language", "StartGame_Disabled", GameLanguage.StartGame_Disabled);
        reader.Write("Language", "StartGame_NotLoggedIn", GameLanguage.StartGame_NotLoggedIn);
        reader.Write("Language", "StartGame_CharacterNotFound", GameLanguage.StartGame_CharacterNotFound);
        reader.Write("Language", "StartGame_NoMapOrStartPoint", GameLanguage.StartGame_NoMapOrStartPoint);

        reader.Write("Language", "WarriorsDes", GameLanguage.WarriorsDes);
        reader.Write("Language", "WizardDes", GameLanguage.WizardDes);
        reader.Write("Language", "TaoistDes", GameLanguage.TaoistDes);
        reader.Write("Language", "AssassinDes", GameLanguage.AssassinDes);
        reader.Write("Language", "ArcherDes", GameLanguage.ArcherDes);

        reader.Write("Language", "DateSent", GameLanguage.DateSent);
        reader.Write("Language", "Send", GameLanguage.Send);
        reader.Write("Language", "Reply", GameLanguage.Reply);
        reader.Write("Language", "Read", GameLanguage.Read);
        reader.Write("Language", "Delete", GameLanguage.Delete);
        reader.Write("Language", "BlockList", GameLanguage.BlockList);
        reader.Write("Language", "EnterMailToName", GameLanguage.EnterMailToName);
        reader.Write("Language", "BeenPoisoned", GameLanguage.BeenPoisoned);
        reader.Write("Language", "AddFriend", GameLanguage.AddFriend);
        reader.Write("Language", "RemoveFriend", GameLanguage.RemoveFriend);
        reader.Write("Language", "FriendMemo", GameLanguage.FriendMemo);
        reader.Write("Language", "FriendMail", GameLanguage.FriendMail);
        reader.Write("Language", "FriendWhisper", GameLanguage.FriendWhisper);
        reader.Write("Language", "FriendEnterAddName", GameLanguage.FriendEnterAddName);
        reader.Write("Language", "FriendEnterBlockName", GameLanguage.FriendEnterBlockName);
        reader.Write("Language", "AddMentor", GameLanguage.AddMentor);
        reader.Write("Language", "RemoveMentorMentee", GameLanguage.RemoveMentorMentee);
        reader.Write("Language", "MentorRequests", GameLanguage.MentorRequests);
        reader.Write("Language", "MentorEnterName", GameLanguage.MentorEnterName);
        reader.Write("Language", "NoMentorship", GameLanguage.NoMentorship);
        reader.Write("Language", "RestedBuff", GameLanguage.RestedBuff);

        reader.Write("Language", "SkillMode_Tilde", GameLanguage.SkillMode_Tilde);
        reader.Write("Language", "SkillMode_Ctrl", GameLanguage.SkillMode_Ctrl);
        reader.Write("Language", "MainDialog_HP", GameLanguage.MainDialog_HP);
        reader.Write("Language", "MainDialog_MP", GameLanguage.MainDialog_MP);

        reader.Write("Language", "Option_HPMPMode1", GameLanguage.Option_HPMPMode1);
        reader.Write("Language", "Option_HPMPMode2", GameLanguage.Option_HPMPMode2);
        reader.Write("Language", "Option_NewMove", GameLanguage.Option_NewMove);
        reader.Write("Language", "Option_OldMove", GameLanguage.Option_OldMove);

        reader.Write("Language", "Keyboard_Layout", GameLanguage.Keyboard_Layout);

        reader.Write("Language", "Chat_Report", GameLanguage.Chat_Report);

        reader.Write("Language", "Ranking_OnlineOnly", GameLanguage.Ranking_OnlineOnly);

        reader.Write("Language", "Keyboard_EnforceStrict", GameLanguage.Keyboard_EnforceStrict);
        reader.Write("Language", "Keyboard_EnforceRelaxed", GameLanguage.Keyboard_EnforceRelaxed);

        reader.Write("Language", "InputKey_Esc", GameLanguage.InputKey_Esc);
        reader.Write("Language", "InputKey_Delete", GameLanguage.InputKey_Delete);
        reader.Write("Language", "InputKey_Enter", GameLanguage.InputKey_Enter);
        reader.Write("Language", "InputKey_Random", GameLanguage.InputKey_Random);

        reader.Write("Language", "ItemTypeWeapon", GameLanguage.ItemTypeWeapon);
        reader.Write("Language", "ItemTypeArmour", GameLanguage.ItemTypeArmour);
        reader.Write("Language", "ItemTypeHelmet", GameLanguage.ItemTypeHelmet);
        reader.Write("Language", "ItemTypeNecklace", GameLanguage.ItemTypeNecklace);
        reader.Write("Language", "ItemTypeBracelet", GameLanguage.ItemTypeBracelet);
        reader.Write("Language", "ItemTypeRing", GameLanguage.ItemTypeRing);
        reader.Write("Language", "ItemTypeAmulet", GameLanguage.ItemTypeAmulet);
        reader.Write("Language", "ItemTypeBelt", GameLanguage.ItemTypeBelt);
        reader.Write("Language", "ItemTypeBoots", GameLanguage.ItemTypeBoots);
        reader.Write("Language", "ItemTypeStone", GameLanguage.ItemTypeStone);
        reader.Write("Language", "ItemTypeTorch", GameLanguage.ItemTypeTorch);
        reader.Write("Language", "ItemTypePotion", GameLanguage.ItemTypePotion);
        reader.Write("Language", "ItemTypeOre", GameLanguage.ItemTypeOre);
        reader.Write("Language", "ItemTypeMeat", GameLanguage.ItemTypeMeat);
        reader.Write("Language", "ItemTypeCraftingMaterial", GameLanguage.ItemTypeCraftingMaterial);
        reader.Write("Language", "ItemTypeScroll", GameLanguage.ItemTypeScroll);
        reader.Write("Language", "ItemTypeGem", GameLanguage.ItemTypeGem);
        reader.Write("Language", "ItemTypeMount", GameLanguage.ItemTypeMount);
        reader.Write("Language", "ItemTypeBook", GameLanguage.ItemTypeBook);
        reader.Write("Language", "ItemTypeScript", GameLanguage.ItemTypeScript);
        reader.Write("Language", "ItemTypeReins", GameLanguage.ItemTypeReins);
        reader.Write("Language", "ItemTypeBells", GameLanguage.ItemTypeBells);
        reader.Write("Language", "ItemTypeSaddle", GameLanguage.ItemTypeSaddle);
        reader.Write("Language", "ItemTypeRibbon", GameLanguage.ItemTypeRibbon);
        reader.Write("Language", "ItemTypeMask", GameLanguage.ItemTypeMask);
        reader.Write("Language", "ItemTypeFood", GameLanguage.ItemTypeFood);
        reader.Write("Language", "ItemTypeHook", GameLanguage.ItemTypeHook);
        reader.Write("Language", "ItemTypeFloat", GameLanguage.ItemTypeFloat);
        reader.Write("Language", "ItemTypeBait", GameLanguage.ItemTypeBait);
        reader.Write("Language", "ItemTypeFinder", GameLanguage.ItemTypeFinder);
        reader.Write("Language", "ItemTypeReel", GameLanguage.ItemTypeReel);
        reader.Write("Language", "ItemTypeFish", GameLanguage.ItemTypeFish);
        reader.Write("Language", "ItemTypeQuest", GameLanguage.ItemTypeQuest);
        reader.Write("Language", "ItemTypeAwakening", GameLanguage.ItemTypeAwakening);
        reader.Write("Language", "ItemTypePets", GameLanguage.ItemTypePets);
        reader.Write("Language", "ItemTypeTransform", GameLanguage.ItemTypeTransform);
        reader.Write("Language", "ItemTypeSealedHero", GameLanguage.ItemTypeSealedHero);

        reader.Write("Language", "ItemGradeCommon", GameLanguage.ItemGradeCommon);
        reader.Write("Language", "ItemGradeRare", GameLanguage.ItemGradeRare);
        reader.Write("Language", "ItemGradeLegendary", GameLanguage.ItemGradeLegendary);
        reader.Write("Language", "ItemGradeMythical", GameLanguage.ItemGradeMythical);
        reader.Write("Language", "ItemGradeHeroic", GameLanguage.ItemGradeHeroic);

        reader.Write("Language", "NoAccountID", GameLanguage.NoAccountID);
        reader.Write("Language", "IncorrectPasswordAccountID", GameLanguage.IncorrectPasswordAccountID);
        reader.Write("Language", "GroupSwitch", GameLanguage.GroupSwitch);
        reader.Write("Language", "GroupAdd", GameLanguage.GroupAdd);
        reader.Write("Language", "GroupRemove", GameLanguage.GroupRemove);
        reader.Write("Language", "GroupAddEnterName", GameLanguage.GroupAddEnterName);
        reader.Write("Language", "GroupRemoveEnterName", GameLanguage.GroupRemoveEnterName);
        reader.Write("Language", "TooHeavyToHold", GameLanguage.TooHeavyToHold);
        reader.Write("Language", "SwitchMarriage", GameLanguage.SwitchMarriage);
        reader.Write("Language", "RequestMarriage", GameLanguage.RequestMarriage);
        reader.Write("Language", "RequestDivorce", GameLanguage.RequestDivorce);
        reader.Write("Language", "MailLover", GameLanguage.MailLover);
        reader.Write("Language", "WhisperLover", GameLanguage.WhisperLover);

        // Skill related
        reader.Write("Language", "Skill_NoSuitableWeapon", GameLanguage.Skill_NoSuitableWeapon);
        reader.Write("Language", "Skill_CannotCast", GameLanguage.Skill_CannotCast);
        reader.Write("Language", "Skill_UseThrusting", GameLanguage.Skill_UseThrusting);
        reader.Write("Language", "Skill_DoNotUseThrusting", GameLanguage.Skill_DoNotUseThrusting);
        reader.Write("Language", "Skill_UseHalfMoon", GameLanguage.Skill_UseHalfMoon);
        reader.Write("Language", "Skill_DoNotUseHalfMoon", GameLanguage.Skill_DoNotUseHalfMoon);
        reader.Write("Language", "Skill_UseCrossHalfMoon", GameLanguage.Skill_UseCrossHalfMoon);
        reader.Write("Language", "Skill_DoNotUseCrossHalfMoon", GameLanguage.Skill_DoNotUseCrossHalfMoon);
        reader.Write("Language", "Skill_UseDoubleSlash", GameLanguage.Skill_UseDoubleSlash);
        reader.Write("Language", "Skill_DoNotUseDoubleSlash", GameLanguage.Skill_DoNotUseDoubleSlash);

        // System messages
        reader.Write("Language", "System_NothingFound", GameLanguage.System_NothingFound);
        reader.Write("Language", "System_PleaseEnterInfo", GameLanguage.System_PleaseEnterInfo);
        reader.Write("Language", "System_PersonObservingLoggedOff", GameLanguage.System_PersonObservingLoggedOff);
        reader.Write("Language", "System_UnknownTypeRequired", GameLanguage.System_UnknownTypeRequired);

        // Group related
        reader.Write("Language", "Group_YouLeft", GameLanguage.Group_YouLeft);
        reader.Write("Language", "Group_PlayerLeft", GameLanguage.Group_PlayerLeft);
        reader.Write("Language", "Group_PlayerJoined", GameLanguage.Group_PlayerJoined);
        reader.Write("Language", "Group_InviteQuestion", GameLanguage.Group_InviteQuestion);

        // Quest related
        reader.Write("Language", "Quest_ShareQuestion", GameLanguage.Quest_ShareQuestion);

        // Item related
        reader.Write("Language", "Item_LocationAt", GameLanguage.Item_LocationAt);
        reader.Write("Language", "Item_NoLongerLoyal", GameLanguage.Item_NoLongerLoyal);
        reader.Write("Language", "Item_DuraDroppedToZero", GameLanguage.Item_DuraDroppedToZero);
        reader.Write("Language", "Item_AddsDurability", GameLanguage.Item_AddsDurability);
        reader.Write("Language", "Item_AddsAccuracy", GameLanguage.Item_AddsAccuracy);
        reader.Write("Language", "Item_AddsASpeed", GameLanguage.Item_AddsASpeed);
        reader.Write("Language", "Item_AddsFreezing", GameLanguage.Item_AddsFreezing);
        reader.Write("Language", "Item_AddsPoison", GameLanguage.Item_AddsPoison);
        reader.Write("Language", "Item_AddsAgility", GameLanguage.Item_AddsAgility);
        reader.Write("Language", "Item_AddsPoisonResist", GameLanguage.Item_AddsPoisonResist);
        reader.Write("Language", "Item_AddsMagicResist", GameLanguage.Item_AddsMagicResist);
        reader.Write("Language", "Item_InstantRun", GameLanguage.Item_InstantRun);
        reader.Write("Language", "Item_Socket", GameLanguage.Item_Socket);
        reader.Write("Language", "Item_SocketEmpty", GameLanguage.Item_SocketEmpty);
        reader.Write("Language", "Item_SocketOpenHint", GameLanguage.Item_SocketOpenHint);
        reader.Write("Language", "Item_SellingPrice", GameLanguage.Item_SellingPrice);
        reader.Write("Language", "Item_CantDropOnDeath", GameLanguage.Item_CantDropOnDeath);
        reader.Write("Language", "Item_CantDrop", GameLanguage.Item_CantDrop);
        reader.Write("Language", "Item_CantUpgrade", GameLanguage.Item_CantUpgrade);
        reader.Write("Language", "Item_CantSell", GameLanguage.Item_CantSell);
        reader.Write("Language", "Item_CantTrade", GameLanguage.Item_CantTrade);
        reader.Write("Language", "Item_CantStore", GameLanguage.Item_CantStore);
        reader.Write("Language", "Item_CantRepair", GameLanguage.Item_CantRepair);

        // Damage types
        reader.Write("Language", "Damage_Miss", GameLanguage.Damage_Miss);
        reader.Write("Language", "Damage_Crit", GameLanguage.Damage_Crit);

        // Error messages
        reader.Write("Language", "Error_CouldNotGetDisplayResolutions", GameLanguage.Error_CouldNotGetDisplayResolutions);
        reader.Write("Language", "Error_GetDisplayResolutionIssue", GameLanguage.Error_GetDisplayResolutionIssue);
        reader.Write("Language", "Error_InvalidClientResolution", GameLanguage.Error_InvalidClientResolution);
    }


    public static void LoadServerLanguage(string languageIniPath)
    {
        if (!File.Exists(languageIniPath))
        {
            SaveServerLanguage(languageIniPath);
            return;
        }
        InIReader reader = new InIReader(languageIniPath);
        GameLanguage.Welcome = reader.ReadString("Language", "Welcome", GameLanguage.Welcome);
        GameLanguage.OnlinePlayers = reader.ReadString("Language", "OnlinePlayers", GameLanguage.OnlinePlayers);
        GameLanguage.LowLevel = reader.ReadString("Language", "LowLevel", GameLanguage.LowLevel);
        GameLanguage.LowGold = reader.ReadString("Language", "LowGold", GameLanguage.LowGold);
        GameLanguage.LowDC = reader.ReadString("Language", "LowDC", GameLanguage.LowDC);
        GameLanguage.LowMC = reader.ReadString("Language", "LowMC", GameLanguage.LowMC);
        GameLanguage.LowSC = reader.ReadString("Language", "LowSC", GameLanguage.LowSC);

        GameLanguage.LevelUp = reader.ReadString("Language", "LevelUp", GameLanguage.LevelUp);

        GameLanguage.WeaponLuck = reader.ReadString("Language", "WeaponLuck", GameLanguage.WeaponLuck);
        GameLanguage.WeaponCurse = reader.ReadString("Language", "WeaponCurse", GameLanguage.WeaponCurse);
        GameLanguage.WeaponNoEffect = reader.ReadString("Language", "WeaponNoEffect", GameLanguage.WeaponNoEffect);

        GameLanguage.InventoryIncreased = reader.ReadString("Language", "InventoryIncreased", GameLanguage.InventoryIncreased);
        GameLanguage.ExpandedStorageExpiresOn = reader.ReadString("Language", "ExpandedStorageExpiresOn", GameLanguage.ExpandedStorageExpiresOn);
        GameLanguage.GameName = reader.ReadString("Language", "GameName", GameLanguage.GameName);
        GameLanguage.FaceToTrade = reader.ReadString("Language", "FaceToTrade", GameLanguage.FaceToTrade);
        GameLanguage.NoTownTeleport = reader.ReadString("Language", "NoTownTeleport", GameLanguage.NoTownTeleport);
        GameLanguage.CanNotRandom = reader.ReadString("Language", "CanNotRandom", GameLanguage.CanNotRandom);
        GameLanguage.CanNotDungeon = reader.ReadString("Language", "CanNotDungeon", GameLanguage.CanNotDungeon);
        GameLanguage.CannotResurrection = reader.ReadString("Language", "CannotResurrection", GameLanguage.CannotResurrection);
        GameLanguage.CanNotDrop = reader.ReadString("Language", "CanNotDrop", GameLanguage.CanNotDrop);

        GameLanguage.NotFemale = reader.ReadString("Language", "NotFemale", GameLanguage.NotFemale);
        GameLanguage.NotMale = reader.ReadString("Language", "NotMale", GameLanguage.NotMale);
        GameLanguage.NotInGuild = reader.ReadString("Language", "NotInGuild", GameLanguage.NotInGuild);
        GameLanguage.NewMail = reader.ReadString("Language", "NewMail", GameLanguage.NewMail);
        GameLanguage.CouldNotFindPlayer = reader.ReadString("Language", "CouldNotFindPlayer", GameLanguage.CouldNotFindPlayer);
        GameLanguage.NoMentorship = reader.ReadString("Language", "NoMentorship", GameLanguage.NoMentorship);
        GameLanguage.NoBagSpace = reader.ReadString("Language", "NoBagSpace", GameLanguage.NoBagSpace);
        GameLanguage.AllowingMentorRequests = reader.ReadString("Language", "AllowingMentorRequests", GameLanguage.AllowingMentorRequests);
        GameLanguage.BlockingMentorRequests = reader.ReadString("Language", "BlockingMentorRequests", GameLanguage.BlockingMentorRequests);

        LoadDatabaseTranslations(languageIniPath);
    }

    public static void SaveServerLanguage(string languageIniPath)
    {
        File.Delete(languageIniPath);
        InIReader reader = new InIReader(languageIniPath);
        reader.Write("Language", "Welcome", GameLanguage.Welcome);
        reader.Write("Language", "OnlinePlayers", GameLanguage.OnlinePlayers);
        reader.Write("Language", "LowLevel", GameLanguage.LowLevel);
        reader.Write("Language", "LowGold", GameLanguage.LowGold);
        reader.Write("Language", "LowDC", GameLanguage.LowDC);
        reader.Write("Language", "LowMC", GameLanguage.LowMC);
        reader.Write("Language", "LowSC", GameLanguage.LowSC);

        reader.Write("Language", "LevelUp", GameLanguage.LevelUp);

        reader.Write("Language", "WeaponLuck", GameLanguage.WeaponLuck);
        reader.Write("Language", "WeaponCurse", GameLanguage.WeaponCurse);
        reader.Write("Language", "WeaponNoEffect", GameLanguage.WeaponNoEffect);

        reader.Write("Language", "InventoryIncreased", GameLanguage.InventoryIncreased);
        reader.Write("Language", "ExpandedStorageExpiresOn", GameLanguage.ExpandedStorageExpiresOn);
        reader.Write("Language", "GameName", GameLanguage.GameName);
        reader.Write("Language", "FaceToTrade", GameLanguage.FaceToTrade);
        reader.Write("Language", "NoTownTeleport", GameLanguage.NoTownTeleport);
        reader.Write("Language", "CanNotRandom", GameLanguage.CanNotRandom);
        reader.Write("Language", "CanNotDungeon", GameLanguage.CanNotDungeon);
        reader.Write("Language", "CannotResurrection", GameLanguage.CannotResurrection);
        reader.Write("Language", "CanNotDrop", GameLanguage.CanNotDrop);

        reader.Write("Language", "NotFemale", GameLanguage.NotFemale);
        reader.Write("Language", "NotMale", GameLanguage.NotMale);
        reader.Write("Language", "NotInGuild", GameLanguage.NotInGuild);
        reader.Write("Language", "NewMail", GameLanguage.NewMail);
        reader.Write("Language", "CouldNotFindPlayer", GameLanguage.CouldNotFindPlayer);
        reader.Write("Language", "NoMentorship", GameLanguage.NoMentorship);
        reader.Write("Language", "NoBagSpace", GameLanguage.NoBagSpace);
        reader.Write("Language", "AllowingMentorRequests", GameLanguage.AllowingMentorRequests);
        reader.Write("Language", "BlockingMentorRequests", GameLanguage.BlockingMentorRequests);
    }

    private static readonly Dictionary<string, string> ItemNameById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, string> MonsterNameById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, string> NpcNameById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, string> MagicNameById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, string> QuestNameById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

    public static string GetItemName(string englishName)
    {
        if (string.IsNullOrEmpty(englishName))
            return englishName ?? string.Empty;

        string id = englishName.Replace(" ", string.Empty).Replace("(", string.Empty).Replace(")", string.Empty);
        string key = "Item_" + id;

        string value;
        if (ItemNameById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
            return value;

        return englishName;
    }

    public static string GetMonsterName(string englishName)
    {
        if (string.IsNullOrEmpty(englishName))
            return englishName ?? string.Empty;

        string id = englishName.Replace(" ", string.Empty).Replace("-", string.Empty);
        string key = "Monster_" + id;

        string value;
        if (MonsterNameById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
            return value;

        return englishName;
    }

    public static string GetNPCName(string englishName)
    {
        if (string.IsNullOrEmpty(englishName))
            return englishName ?? string.Empty;

        string id = englishName.Replace(" ", string.Empty).Replace("-", string.Empty);
        string key = "NPC_" + id;

        string value;
        if (NpcNameById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
            return value;

        return englishName;
    }

    public static string GetMagicName(string englishName)
    {
        if (string.IsNullOrEmpty(englishName))
            return englishName ?? string.Empty;

        string id = englishName.Replace(" ", string.Empty).Replace("-", string.Empty);
        string key = "Magic_" + id;

        string value;
        if (MagicNameById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
            return value;

        return englishName;
    }

    public static string GetQuestName(string englishName)
    {
        if (string.IsNullOrEmpty(englishName))
            return englishName ?? string.Empty;

        string id = englishName.Replace(" ", string.Empty).Replace("-", string.Empty);
        string key = "Quest_" + id;

        string value;
        if (QuestNameById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
            return value;

        return englishName;
    }

    private static void LoadDatabaseTranslations(string languageIniPath)
    {
        ItemNameById.Clear();
        MonsterNameById.Clear();
        NpcNameById.Clear();
        MagicNameById.Clear();
        QuestNameById.Clear();

        if (!File.Exists(languageIniPath))
            return;

        string[] lines;
        try
        {
            lines = File.ReadAllLines(languageIniPath);
        }
        catch
        {
            return;
        }

        bool inLanguageSection = false;

        foreach (string rawLine in lines)
        {
            string line = rawLine.Trim();
            if (string.IsNullOrEmpty(line)) continue;

            if (line.StartsWith("[", StringComparison.Ordinal) && line.EndsWith("]", StringComparison.Ordinal))
            {
                inLanguageSection = string.Equals(line, "[Language]", StringComparison.OrdinalIgnoreCase);
                continue;
            }

            if (!inLanguageSection) continue;

            int index = line.IndexOf('=');
            if (index <= 0 || index >= line.Length - 1) continue;

            string key = line.Substring(0, index).Trim();
            string value = line.Substring(index + 1);

            if (string.IsNullOrEmpty(key)) continue;

            if (key.StartsWith("Item_", StringComparison.OrdinalIgnoreCase))
            {
                ItemNameById[key] = value;
            }
            else if (key.StartsWith("Monster_", StringComparison.OrdinalIgnoreCase))
            {
                MonsterNameById[key] = value;
            }
            else if (key.StartsWith("NPC_", StringComparison.OrdinalIgnoreCase))
            {
                NpcNameById[key] = value;
            }
            else if (key.StartsWith("Magic_", StringComparison.OrdinalIgnoreCase))
            {
                MagicNameById[key] = value;
            }
            else if (key.StartsWith("Quest_", StringComparison.OrdinalIgnoreCase))
            {
                QuestNameById[key] = value;
            }
        }
    }
}

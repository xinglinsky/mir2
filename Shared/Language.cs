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
                         MiniMap_Hint = "MiniMap ({0})",
                         Inspect_InviteToGroup = "Invite to Group",
                         Inspect_AddToFriends = "Add to Friends List",
                         Inspect_SendMail = "Send Mail",
                         Inspect_Trade = "Trade",
                         Inspect_Observe = "Observe",
                         GuildTerritory_None = "None",
                         GuildTerritory_StatusAvailable = "Available",
                         GuildTerritory_StatusForSale = "For Sale",
                         GuildTerritory_StatusSalePending = "Sale pending",
                         GuildTerritory_StatusUnavailable = "Unavailable",
                         GuildTerritory_OwnerAndPrefix = " and ",
                         Mail_ReportBug = "Report Bug",
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
                         ClassName_Warrior = "Warrior",
                         ClassName_Wizard = "Wizard",
                         ClassName_Taoist = "Taoist",
                         ClassName_Assassin = "Assassin",
                         ClassName_Archer = "Archer",

                         Gold = "Gold",
                         Credit = "Credit",
                         GameShop_BuyWithGold = "Buy with Gold",
                         GameShop_BuyWithCredits = "Buy with Credits",
                         GameShop_BuyWithGoldHint = "Buy item(s) with Gold.",
                         GameShop_BuyWithCreditsHint = "Buy item(s) with Credits.",
                         GameShop_ShowAll = "Show All",
                         GameShop_PageText = "{0} / {1}",
                         GameShop_BuyConfirmCredits = "Are you sure you would like to buy {1} x \n{0}({3}) for {2} Credits?",
                         GameShop_BuyConfirmGold = "Are you sure you would like to buy {1} x \n{0}({3}) for {2} Gold?",
                         GameShop_SelectPaymentTypeRequired = "You MUST select a payment type!",
                         GameShop_CannotAffordItem = "You can't afford the selected item.",

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
                         Disconnect_LoggedInElsewhere = "Disconnected: Another user logged onto your account.",
                         Disconnect_PacketError = "Disconnected: Packet Error.",
                         Disconnect_ServerCrashed = "Disconnected: Server Crashed.",
                         Disconnect_KickedByAdmin = "Disconnected: Kicked by Admin.",
                         Disconnect_MaxConnectionsReached = "Disconnected: Maximum connections reached.",
                         Disconnect_LostConnection = "Lost connection with the server.",
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
                         AttemptingConnectFirst = "Attempting to connect to the server.",
                         AttemptingConnect = "Attempting to connect to the server.{0}Attempt:{1}",

                         CreatingCharactersDisabled = "Creating new characters is currently disabled.",
                         InvalidCharacterName = "Your Character Name is not acceptable.",
                         NoClass = "The class you selected does not exist. Contact a GM for assistance.",
                         ToManyCharacters = "You cannot make anymore then {0} Characters.",
                         CharacterNameExists = "A Character with this name already exists.",

                         Hero_CreatingDisabled = "Creating new heroes is currently disabled.",
                         Hero_InvalidName = "Your Hero Name is not acceptable.",
                         Hero_GenderNotExist = "The gender you selected does not exist.\n Contact a GM for assistance.",
                         Hero_ClassNotExist = "The class you selected does not exist.\n Contact a GM for assistance.",
                         Hero_TooManyHeroes = "You cannot make anymore Heroes.",
                         Hero_NameExists = "A Character with this name already exists.",
                         Hero_NoBagSpace = "No bag space.",
                         Hero_CreatedSuccessfully = "Hero created successfully.",
                         Hero_Prefix = "(Hero) ",

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
                         Login_SendingClientVersion = "Sending Client Version.",
                         Login_DescAccountID = " Description: Account ID.\n Accepted characters: a-z A-Z 0-9.\n Length: between {0} and {1} characters.",
                         Login_DescPassword = " Description: Password.\n Accepted characters: a-z A-Z 0-9.\n Length: between {0} and {1} characters.",
                         Login_DescEMail = " Description: E-Mail Address.\n Format: Example@Example.Com.\n Max Length: 50 characters.\n Optional Field.",
                         Login_DescUserName = " Description: User Name.\n Accepted characters: All.\n Length: between 0 and 20 characters.\n Optional Field.",
                         Login_DescBirthDate = " Description: Birth Date.\n Format: {0}.\n Length: 10 characters.\n Optional Field.",
                         Login_DescQuestion = " Description: Secret Question.\n Accepted characters: All.\n Length: between 0 and 30 characters.\n Optional Field.",
                         Login_DescAnswer = " Description: Secret Answer.\n Accepted characters: All.\n Length: between 0 and 30 characters.\n Optional Field.",

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
                         Mail_TitleType = "TYPE",
                         Mail_TitleSender = "SENDER",
                         Mail_TitleMessage = "MESSAGE",
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
                         Trade_DealCancelledFaceOther = "Deal cancelled.\r\nTo deal correctly you must face the other party.",
                         Trade_Request = "Player {0} has requested to trade with you.",
                         Item_CannotDrop = "You cannot drop {0}",
                         Creature_NameLengthInvalid = "Creature name must be between {0} and {1} characters.",
                         Creature_VerificationFailed = "Verification Failed!!",
                         Creature_EnterName = "Please give your creature a name.",
                         Creature_EnterNewName = "Please enter a new name for the creature.",
                         Creature_EnterNameForVerification = "Please enter the creature's name for verification.",
                         Keyboard_ResetDefault = "Keyboard settings have been reset back to default.",
                         Mail_DeleteParcelWithItemsConfirm = "This parcel contains items or gold. Are you sure you want to delete it?",
                         Mail_NoParcels = "No parcels to collect.",
                         Mail_AllParcelsCollected = "All parcels have been collected.",
                         Reincarnation_Request = "Would you like to be revived?",
                         Potion_UseSpecialConfirm = "Are you sure you want to use this Potion?",
                         Item_CombineConfirm = "Do you want to try and combine these items?",
                         ItemRental_CancelledFaceOther = "Item rental cancelled.\r\nTo complete item rental please face the other party throughout the transaction.",
                         Guild_JoinRequest = "Do you want to join the {0} guild?",
                         Guild_CreateEnterName = "Please enter a guild name, length must be 3~20 characters.",
                         Guild_WarEnterName = "Please enter the guild you would like to go to war with.",
                         Item_SoulboundTo = "Soulbound to: {0}",
                         Item_Cursed = "Cursed",
                         Item_GemCannotUse = "Cannot be used on any item.",
                         Item_GemCanUseOn = "Can be used on: ",
                         Item_GemUseOnWeapon = "-Weapon",
                         Item_GemUseOnArmour = "-Armour",
                         Item_GemUseOnHelmet = "-Helmet",
                         Item_GemUseOnNecklace = "-Necklace",
                         Item_GemUseOnBracelet = "-Bracelet",
                         Item_GemUseOnRing = "-Ring",
                         Item_GemUseOnAmulet = "-Amulet",
                         Item_GemUseOnBelt = "-Belt",
                         Item_GemUseOnBoots = "-Boots",
                         Item_GemUseOnStone = "-Stone",
                         Item_GemUseOnCandle = "-Candle",
                         Item_SoulBindsOnEquip = "SoulBinds on equip",
                         Item_CannotBeUsedByHero = "Cannot be used by Hero",
                         Item_ExpiresIn = "Expires in {0}",
                         Item_Expired = "Expired",
                         Item_SealedFor = "Sealed for {0}",
                         Item_RentalFrom = "Item rented from: {0}",
                         Item_RentalExpiresIn = "Rental expires in: {0}",
                         Item_RentalExpired = "Rental expired",
                         Item_RentalLockExpiresIn = "Rental lock expires in: {0}",
                         Item_RentalLockExpired = "Rental lock expired",
                         Item_CantSpecialRepair = "Can't special repair",
                         Item_BreaksOnDeath = "Breaks on death",
                         Item_DestroyedWhenDropped = "Destroyed when dropped",
                         Item_CannotBeWeddingRing = "Cannot be a Wedding Ring",
                         Item_GemHint_RepairPartialWeaponAccessory = "Hold CTRL and left click to partially repair\nweapons and accessory items.",
                         Item_GemHint_RepairPartialArmourDrapery = "Hold CTRL and left click to partially repair\narmour and drapery items.",
                         Item_GemHint_CombineMaybeDestroy = "Hold CTRL and left click to combine with an item.\nHas chance to destroy combining item.",
                         Item_GemHint_CombineNoDestroy = "Hold CTRL and left click to combine with an item.\nWill NOT destroy combining item.",
                         Item_GemHint_RepairFullWeaponAccessory = "Hold CTRL and left click to completely repair\nweapons and accessory items.",
                         Item_GemHint_RepairFullArmourDrapery = "Hold CTRL and left click to completely repair\narmour and drapery items.",
                         Item_GemHint_SealItem = "Hold CTRL and left click to seal an item.",
                         Item_CreditScroll_AddCredits = "Adds {0} Credits to your Account.",
                         Item_CreatedByGameMaster = "Created by Game Master",
                         Mail_GoldLabel = "Gold: {0}",
                         Marriage_Request = "{0} has asked for your hand in marriage.",
                         Divorce_Request = "{0} has requested a divorce",
                         Mentor_Request = "{0} (Level {1}) has requested you teach him the ways of the {2}.",
                         Awakening_NotEnoughMaterials = "You have not supplied enough materials.",
                         Awakening_AlreadyMaxLevel = "Awakening already at maximum level.",
                         Awakening_CannotAwaken = "Cannot awaken this item.",
                         BigMap_TeleportToNPC = "Teleport to this NPC for {0} Gold?",
                         BigMap_PathNotFound = "Could not find suitable path.",
                         BigMap_SearchForNPCs = "Search for NPCs",
                         System_PlayerNotOnline = "Player is not online",
                         TrustMerchant_GetBackUnsold = "{0} has not sold, Are you sure you want to get it back?",
                         TrustMerchant_BuyConfirm = "Are you sure you want to buy {0} for {1:#,##0} {2}?",
                         TrustMerchant_BidConfirm = "Are you sure you want to bid {0:#,##0} Gold for {1}?",
                         TrustMerchant_SearchCooldown = "You can search again after {0} seconds.",
                         TrustMerchant_FailDead = "You cannot use the TrustMerchant when dead.",
                         TrustMerchant_FailNotUsing = "You cannot buy from the TrustMerchant without using.",
                         TrustMerchant_FailSold = "This item has already been sold.",
                         TrustMerchant_FailExpired = "This item has Expired and cannot be brought.",
                         TrustMerchant_FailWeightSpace = "You do not have enough weight or space spare to buy this item.",
                         TrustMerchant_FailOwnItems = "You cannot buy your own items.",
                         TrustMerchant_FailTooFar = "You are too far away from the Trust Merchant.",
                         TrustMerchant_FailHoldGold = "You cannot hold enough gold to get your sale.",
                         TrustMerchant_FailMinBid = "This item has not met the minimum bid yet.",
                         TrustMerchant_FailAuctionEnded = "Auction has already ended for this item.",
                         TrustMerchant_Title_SalePrice = "SALE PRICE",
                         TrustMerchant_Title_SellItem = "SELL ITEM",
                         TrustMerchant_Title_Item = "ITEM",
                         TrustMerchant_Title_Price = "PRICE",
                         TrustMerchant_Title_Expiry = "EXPIRY",
                         TrustMerchant_Title_PriceBid = "PRICE / BID",
                         TrustMerchant_Title_SellerExpiry = "SELLER / EXPIRY",
                         TrustMerchant_Title_StartingBid = "STARTING BID",
                         TrustMerchant_Title_HighestBid = "HIGHEST BID",
                         TrustMerchant_Title_EndDate = "END DATE",
                         TrustMerchant_ConsignHelp = "1. Consignment is {0} gold per item \r\n\r\n2. 1% of sale price is paid to Trust Merchant at sale end\r\n\r\n3. Maximum {1} days of item sale registration until item is removed\r\n\r\n4. Maximum of unlimited items allowed for sale\r\n\r\n5. Sale price can be set between: {2} - {3} gold",
                         TrustMerchant_AuctionHelp = "1. Auction cost is {0} gold, max starting bid is {1} gold per item \r\n\r\n2. 1% of final bid price is paid to Trust Merchant at auction end\r\n\r\n3. Maximum {2} days of item sale registration, afterwards the item will be sent to highest bidder\r\n\r\n4. Maximum of unlimited items allowed for auction\r\n\r\n",
                         TrustMerchant_Filter_All = "Show All Items",
                         TrustMerchant_Filter_Weapon = "Weapon Items",
                         TrustMerchant_Filter_Drapery = "Drapery Items",
                         TrustMerchant_Filter_Accessory = "Accessory Items",
                         TrustMerchant_Filter_Consumable = "Consumable Items",
                         TrustMerchant_Filter_Enhancement = "Enhancement",
                         TrustMerchant_Filter_Book = "Books",
                         TrustMerchant_Filter_Craft = "Craft Items",
                         TrustMerchant_Filter_Drapery_Armour = "Armour",
                         TrustMerchant_Filter_Drapery_Helmet = "Helmet",
                         TrustMerchant_Filter_Drapery_Belt = "Belt",
                         TrustMerchant_Filter_Drapery_Boots = "Boots",
                         TrustMerchant_Filter_Drapery_Stone = "Stone",
                         TrustMerchant_Filter_Accessory_Necklaces = "Necklaces",
                         TrustMerchant_Filter_Accessory_Bracelets = "Bracelets",
                         TrustMerchant_Filter_Accessory_Rings = "Rings",
                         TrustMerchant_Filter_Consumable_Recovery = "Recovery Pots",
                         TrustMerchant_Filter_Consumable_Buff = "Buff Pots",
                         TrustMerchant_Filter_Consumable_Scrolls = "Scrolls / Oils",
                         TrustMerchant_Filter_Consumable_Misc = "Misc Items",
                         TrustMerchant_Filter_Enhancement_Gems = "Gems",
                         TrustMerchant_Filter_Enhancement_Orbs = "Orbs",
                         TrustMerchant_Filter_Book_Warrior = "Warrior",
                         TrustMerchant_Filter_Book_Wizard = "Wizard",
                         TrustMerchant_Filter_Book_Taoist = "Taoist",
                         TrustMerchant_Filter_Book_Assassin = "Assassin",
                         TrustMerchant_Filter_Book_Archer = "Archer",
                         TrustMerchant_Filter_Craft_Materials = "Materials",
                         TrustMerchant_Filter_Craft_Meat = "Meat",
                         TrustMerchant_Filter_Craft_Ore = "Ore",
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
                         Ranking_AllHint = "Overall TOP 20",
                         Ranking_WarriorHint = "TOP 20 Warriors",
                         Ranking_WizardHint = "TOP 20 Wizards",
                         Ranking_TaoistHint = "TOP 20 Taoists",
                         Ranking_AssassinHint = "TOP 20 Assasins",
                         Ranking_ArcherHint = "TOP 20 Archers",
                         Ranking_NotListed = "Not Listed",
                         Ranking_Ranked = "Ranked: {0}",

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
                         Guild_Buff_InsufficientPoints = "Insufficient points available.",
                         Guild_Buff_GuildLevelTooLow = "Guild level too low.",
                         Guild_Buff_StillActive = "Buff is still active.",
                         Guild_Buff_InsufficientFunds = "Insufficient guild funds.",
                         Guild_EditRank = "Edit Rank",
                         Guild_SelectRank = "Select Rank",
                         Guild_RankNoBuffPermission = "Guild rank does not allow buff activation.",
                         Guild_ChangeRankConfirm = "Are you sure you want to change the rank of {0} to {1}?",
                         Guild_KickMemberConfirm = "Are you sure you want to kick {0}?",
                         Guild_CreateRankConfirm = "Are you sure you want to create a new rank?",
                         Guild_MemberLoggedOn = "{0} logged on.",
                         Guild_MemberJoined = "{0} joined guild.",
                         Guild_MemberKicked = "{0} got removed from the guild.",
                         Guild_MemberLeft = "{0} left the guild.",
                         Guild_DonatedToFund = "{0} donated {1} gold to guild funds.",
                         Guild_RetrievedFromFund = "{0} retrieved {1} gold from guild funds.",

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
                         Guild_InvalidNameBackslash = "You cannot use the \\ sign in a guildname!",
                         TooHeavyToHold = "It is too heavy to Hold.",
                         TooHeavyToWear = "It is too heavy to wear.",
                         Chat_ItemLinkTooLong = "Unable to link item, message exceeds allowed length",
                         Item_NoRoomToSplitStack = "No room to split stack.",
                         Item_CannotSwapItems = "You cannot swap items.",
                         Guild_InsufficientRetrieveItems = "Insufficient rights to retrieve items.",
                         Guild_InsufficientStoreItems = "Insufficient rights to store items.",
                         Rental_CannotRemoveLockedItem = "Unable to remove locked item, cancel item rental and try again.",
                         Mail_CannotMailItem = "You cannot mail this item.",
                         Item_TooHeavyToTransfer = "Too heavy to transfer.",
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
                         System_TargetTooFar = "Target is too far.",
                         System_ObservationDisabled = "That player has disabled observation.",

                         // Group related
                         Group_YouLeft = "You have left the group.",
                         Group_PlayerLeft = "{0} has left the group.",
                         Group_PlayerJoined = "{0} has joined the group.",
                         Group_InviteQuestion = "Do you want to group with {0}?",


                         // Quest related
                         Quest_ShareQuestion = "{0} would like to share a quest with you. Do you accept?",
                         Quest_TaskTitle = "Tasks",
                         Quest_ProgressTitle = "Progress",
                         Quest_ReturnTitle = "Quest Return",
                         Quest_TimeLimitTitle = "Time Limit",
                         Quest_SelectRewardRequired = "You must select a reward item.",
                         Quest_CancelConfirm = "Are you sure you want to cancel this quest?",

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
                         Item_WarriorCannotUse = "Warriors cannot use this item.",
                         Item_WizardCannotUse = "Wizards cannot use this item.",
                         Item_TaoistCannotUse = "Taoists cannot use this item.",
                         Item_AssassinCannotUse = "Assassins cannot use this item.",
                         Item_ArcherCannotUse = "Archers cannot use this item.",
                         Req_NotEnoughAC = "You do not have enough AC.",
                         Req_NotEnoughMAC = "You do not have enough MAC.",
                         Req_MaxLevelExceeded = "You have exceeded the maximum level.",
                         Req_NotEnoughBaseAC = "You do not have enough Base AC.",
                         Req_NotEnoughBaseMAC = "You do not have enough Base MAC.",
                         Req_NotEnoughBaseDC = "You do not have enough Base DC.",
                         Req_NotEnoughBaseMC = "You do not have enough Base MC.",
                         Req_NotEnoughBaseSC = "You do not have enough Base SC.",
                         Req_NoMountEquipped = "You do not have a mount equipped.",
                         Req_NoFishingRodEquipped = "You do not have a fishing rod equipped.",

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
                         NPC_NotEnoughPearls = "You do not have enough Pearls.",
                         NPC_CannotSellItem = "Cannot sell this item.",
                         NPC_CannotCarryMoreGold = "Cannot carry anymore gold.",
                         NPC_CannotRepairItem = "Cannot repair this item.",
                         NPC_CannotConsignItem = "Cannot consign this item.",
                         NPC_NotEnoughGold = "You do not have enough gold.",
                         NPC_MissingToolsOrIngredients = "You do not have the required tools or ingredients.",
                         NPC_SellPrefix = "Sale: ",
                         NPC_RepairPrefix = "Repair: ",
                         NPC_SpecialRepairPrefix = "S. Repair: ",
                         NPC_ConsignPrefix = "Consignment: ",
                         NPC_DisassembleWarning = "Item will be Destroyed\n\n\n\n\n\n\n\n         ",
                         NPC_DowngradePrefix = "Downgrade: ",
                         NPC_ResetPrefix = "Reset: ",
                         NPC_RefinePrefix = "Refine: ",
                         NPC_CheckRefine = "Check Refine",
                         NPC_ReplaceWedRingPrefix = "Replace: ",
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
        GameLanguage.MiniMap_Hint = reader.ReadString("Language", "MiniMap_Hint", GameLanguage.MiniMap_Hint);
        GameLanguage.Inspect_InviteToGroup = reader.ReadString("Language", "Inspect_InviteToGroup", GameLanguage.Inspect_InviteToGroup);
        GameLanguage.Inspect_AddToFriends = reader.ReadString("Language", "Inspect_AddToFriends", GameLanguage.Inspect_AddToFriends);
        GameLanguage.Inspect_SendMail = reader.ReadString("Language", "Inspect_SendMail", GameLanguage.Inspect_SendMail);
        GameLanguage.Inspect_Trade = reader.ReadString("Language", "Inspect_Trade", GameLanguage.Inspect_Trade);
        GameLanguage.Inspect_Observe = reader.ReadString("Language", "Inspect_Observe", GameLanguage.Inspect_Observe);
        GameLanguage.GuildTerritory_None = reader.ReadString("Language", "GuildTerritory_None", GameLanguage.GuildTerritory_None);
        GameLanguage.GuildTerritory_StatusAvailable = reader.ReadString("Language", "GuildTerritory_StatusAvailable", GameLanguage.GuildTerritory_StatusAvailable);
        GameLanguage.GuildTerritory_StatusForSale = reader.ReadString("Language", "GuildTerritory_StatusForSale", GameLanguage.GuildTerritory_StatusForSale);
        GameLanguage.GuildTerritory_StatusSalePending = reader.ReadString("Language", "GuildTerritory_StatusSalePending", GameLanguage.GuildTerritory_StatusSalePending);
        GameLanguage.GuildTerritory_StatusUnavailable = reader.ReadString("Language", "GuildTerritory_StatusUnavailable", GameLanguage.GuildTerritory_StatusUnavailable);
        GameLanguage.GuildTerritory_OwnerAndPrefix = reader.ReadString("Language", "GuildTerritory_OwnerAndPrefix", GameLanguage.GuildTerritory_OwnerAndPrefix);
        GameLanguage.Mail_ReportBug = reader.ReadString("Language", "Mail_ReportBug", GameLanguage.Mail_ReportBug);
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
        GameLanguage.GameName = reader.ReadString("Language", "GameName", GameLanguage.GameName);
        GameLanguage.ClassName_Warrior = reader.ReadString("Language", "ClassName_Warrior", GameLanguage.ClassName_Warrior);
        GameLanguage.ClassName_Wizard = reader.ReadString("Language", "ClassName_Wizard", GameLanguage.ClassName_Wizard);
        GameLanguage.ClassName_Taoist = reader.ReadString("Language", "ClassName_Taoist", GameLanguage.ClassName_Taoist);
        GameLanguage.ClassName_Assassin = reader.ReadString("Language", "ClassName_Assassin", GameLanguage.ClassName_Assassin);
        GameLanguage.ClassName_Archer = reader.ReadString("Language", "ClassName_Archer", GameLanguage.ClassName_Archer);

        GameLanguage.LowLevel = reader.ReadString("Language", "LowLevel", GameLanguage.LowLevel);
        GameLanguage.LowGold = reader.ReadString("Language", "LowGold", GameLanguage.LowGold);
        GameLanguage.LowDC = reader.ReadString("Language", "LowDC", GameLanguage.LowDC);
        GameLanguage.LowMC = reader.ReadString("Language", "LowMC", GameLanguage.LowMC);
        GameLanguage.LowSC = reader.ReadString("Language", "LowSC", GameLanguage.LowSC);
        GameLanguage.NPC_NotEnoughPearls = reader.ReadString("Language", "NPC_NotEnoughPearls", GameLanguage.NPC_NotEnoughPearls);
        GameLanguage.NPC_CannotSellItem = reader.ReadString("Language", "NPC_CannotSellItem", GameLanguage.NPC_CannotSellItem);
        GameLanguage.NPC_CannotCarryMoreGold = reader.ReadString("Language", "NPC_CannotCarryMoreGold", GameLanguage.NPC_CannotCarryMoreGold);
        GameLanguage.NPC_CannotRepairItem = reader.ReadString("Language", "NPC_CannotRepairItem", GameLanguage.NPC_CannotRepairItem);
        GameLanguage.NPC_CannotConsignItem = reader.ReadString("Language", "NPC_CannotConsignItem", GameLanguage.NPC_CannotConsignItem);
        GameLanguage.NPC_NotEnoughGold = reader.ReadString("Language", "NPC_NotEnoughGold", GameLanguage.NPC_NotEnoughGold);
        GameLanguage.NPC_MissingToolsOrIngredients = reader.ReadString("Language", "NPC_MissingToolsOrIngredients", GameLanguage.NPC_MissingToolsOrIngredients);
        GameLanguage.NPC_SellPrefix = reader.ReadString("Language", "NPC_SellPrefix", GameLanguage.NPC_SellPrefix);
        GameLanguage.NPC_RepairPrefix = reader.ReadString("Language", "NPC_RepairPrefix", GameLanguage.NPC_RepairPrefix);
        GameLanguage.NPC_SpecialRepairPrefix = reader.ReadString("Language", "NPC_SpecialRepairPrefix", GameLanguage.NPC_SpecialRepairPrefix);
        GameLanguage.NPC_ConsignPrefix = reader.ReadString("Language", "NPC_ConsignPrefix", GameLanguage.NPC_ConsignPrefix);
        GameLanguage.NPC_DisassembleWarning = reader.ReadString("Language", "NPC_DisassembleWarning", GameLanguage.NPC_DisassembleWarning);
        GameLanguage.NPC_DowngradePrefix = reader.ReadString("Language", "NPC_DowngradePrefix", GameLanguage.NPC_DowngradePrefix);
        GameLanguage.NPC_ResetPrefix = reader.ReadString("Language", "NPC_ResetPrefix", GameLanguage.NPC_ResetPrefix);
        GameLanguage.NPC_RefinePrefix = reader.ReadString("Language", "NPC_RefinePrefix", GameLanguage.NPC_RefinePrefix);
        GameLanguage.NPC_CheckRefine = reader.ReadString("Language", "NPC_CheckRefine", GameLanguage.NPC_CheckRefine);
        GameLanguage.NPC_ReplaceWedRingPrefix = reader.ReadString("Language", "NPC_ReplaceWedRingPrefix", GameLanguage.NPC_ReplaceWedRingPrefix);

        GameLanguage.Gold = reader.ReadString("Language", "Gold", GameLanguage.Gold);
        GameLanguage.Credit = reader.ReadString("Language", "Credit", GameLanguage.Credit);

        GameLanguage.GameShop_BuyWithGold = reader.ReadString("Language", "GameShop_BuyWithGold", GameLanguage.GameShop_BuyWithGold);
        GameLanguage.GameShop_BuyWithCredits = reader.ReadString("Language", "GameShop_BuyWithCredits", GameLanguage.GameShop_BuyWithCredits);
        GameLanguage.GameShop_BuyWithGoldHint = reader.ReadString("Language", "GameShop_BuyWithGoldHint", GameLanguage.GameShop_BuyWithGoldHint);
        GameLanguage.GameShop_BuyWithCreditsHint = reader.ReadString("Language", "GameShop_BuyWithCreditsHint", GameLanguage.GameShop_BuyWithCreditsHint);
        GameLanguage.GameShop_ShowAll = reader.ReadString("Language", "GameShop_ShowAll", GameLanguage.GameShop_ShowAll);
        GameLanguage.GameShop_PageText = reader.ReadString("Language", "GameShop_PageText", GameLanguage.GameShop_PageText);
        GameLanguage.GameShop_BuyConfirmCredits = reader.ReadString("Language", "GameShop_BuyConfirmCredits", GameLanguage.GameShop_BuyConfirmCredits);
        GameLanguage.GameShop_BuyConfirmGold = reader.ReadString("Language", "GameShop_BuyConfirmGold", GameLanguage.GameShop_BuyConfirmGold);
        GameLanguage.GameShop_SelectPaymentTypeRequired = reader.ReadString("Language", "GameShop_SelectPaymentTypeRequired", GameLanguage.GameShop_SelectPaymentTypeRequired);
        GameLanguage.GameShop_CannotAffordItem = reader.ReadString("Language", "GameShop_CannotAffordItem", GameLanguage.GameShop_CannotAffordItem);

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
        GameLanguage.Disconnect_LoggedInElsewhere = reader.ReadString("Language", "Disconnect_LoggedInElsewhere", GameLanguage.Disconnect_LoggedInElsewhere);
        GameLanguage.Disconnect_PacketError = reader.ReadString("Language", "Disconnect_PacketError", GameLanguage.Disconnect_PacketError);
        GameLanguage.Disconnect_ServerCrashed = reader.ReadString("Language", "Disconnect_ServerCrashed", GameLanguage.Disconnect_ServerCrashed);
        GameLanguage.Disconnect_KickedByAdmin = reader.ReadString("Language", "Disconnect_KickedByAdmin", GameLanguage.Disconnect_KickedByAdmin);
        GameLanguage.Disconnect_MaxConnectionsReached = reader.ReadString("Language", "Disconnect_MaxConnectionsReached", GameLanguage.Disconnect_MaxConnectionsReached);
        GameLanguage.Disconnect_LostConnection = reader.ReadString("Language", "Disconnect_LostConnection", GameLanguage.Disconnect_LostConnection);

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
        GameLanguage.AttemptingConnectFirst = reader.ReadString("Language", "AttemptingConnectFirst", GameLanguage.AttemptingConnectFirst);
        GameLanguage.AttemptingConnect = reader.ReadString("Language", "AttemptingConnect", GameLanguage.AttemptingConnect);

        GameLanguage.CreatingCharactersDisabled = reader.ReadString("Language", "CreatingCharactersDisabled", GameLanguage.CreatingCharactersDisabled);
        GameLanguage.InvalidCharacterName = reader.ReadString("Language", "InvalidCharacterName", GameLanguage.InvalidCharacterName);
        GameLanguage.NoClass = reader.ReadString("Language", "NoClass", GameLanguage.NoClass);
        GameLanguage.ToManyCharacters = reader.ReadString("Language", "ToManyCharacters", GameLanguage.ToManyCharacters);
        GameLanguage.CharacterNameExists = reader.ReadString("Language", "CharacterNameExists", GameLanguage.CharacterNameExists);

        GameLanguage.Hero_CreatingDisabled = reader.ReadString("Language", "Hero_CreatingDisabled", GameLanguage.Hero_CreatingDisabled);
        GameLanguage.Hero_InvalidName = reader.ReadString("Language", "Hero_InvalidName", GameLanguage.Hero_InvalidName);
        GameLanguage.Hero_GenderNotExist = reader.ReadString("Language", "Hero_GenderNotExist", GameLanguage.Hero_GenderNotExist);
        GameLanguage.Hero_ClassNotExist = reader.ReadString("Language", "Hero_ClassNotExist", GameLanguage.Hero_ClassNotExist);
        GameLanguage.Hero_TooManyHeroes = reader.ReadString("Language", "Hero_TooManyHeroes", GameLanguage.Hero_TooManyHeroes);
        GameLanguage.Hero_NameExists = reader.ReadString("Language", "Hero_NameExists", GameLanguage.Hero_NameExists);
        GameLanguage.Hero_NoBagSpace = reader.ReadString("Language", "Hero_NoBagSpace", GameLanguage.Hero_NoBagSpace);
        GameLanguage.Hero_CreatedSuccessfully = reader.ReadString("Language", "Hero_CreatedSuccessfully", GameLanguage.Hero_CreatedSuccessfully);
        GameLanguage.Hero_Prefix = reader.ReadString("Language", "Hero_Prefix", GameLanguage.Hero_Prefix);

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
        GameLanguage.Login_SendingClientVersion = reader.ReadString("Language", "Login_SendingClientVersion", GameLanguage.Login_SendingClientVersion);
        GameLanguage.Login_DescAccountID = reader.ReadString("Language", "Login_DescAccountID", GameLanguage.Login_DescAccountID);
        GameLanguage.Login_DescPassword = reader.ReadString("Language", "Login_DescPassword", GameLanguage.Login_DescPassword);
        GameLanguage.Login_DescEMail = reader.ReadString("Language", "Login_DescEMail", GameLanguage.Login_DescEMail);
        GameLanguage.Login_DescUserName = reader.ReadString("Language", "Login_DescUserName", GameLanguage.Login_DescUserName);
        GameLanguage.Login_DescBirthDate = reader.ReadString("Language", "Login_DescBirthDate", GameLanguage.Login_DescBirthDate);
        GameLanguage.Login_DescQuestion = reader.ReadString("Language", "Login_DescQuestion", GameLanguage.Login_DescQuestion);
        GameLanguage.Login_DescAnswer = reader.ReadString("Language", "Login_DescAnswer", GameLanguage.Login_DescAnswer);

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
        GameLanguage.Mail_TitleType = reader.ReadString("Language", "Mail_TitleType", GameLanguage.Mail_TitleType);
        GameLanguage.Mail_TitleSender = reader.ReadString("Language", "Mail_TitleSender", GameLanguage.Mail_TitleSender);
        GameLanguage.Mail_TitleMessage = reader.ReadString("Language", "Mail_TitleMessage", GameLanguage.Mail_TitleMessage);
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
        GameLanguage.Friend_RemoveConfirm = reader.ReadString("Language", "Friend_RemoveConfirm", GameLanguage.Friend_RemoveConfirm);
        GameLanguage.Trade_DealCancelledFaceOther = reader.ReadString("Language", "Trade_DealCancelledFaceOther", GameLanguage.Trade_DealCancelledFaceOther);
        GameLanguage.Trade_Request = reader.ReadString("Language", "Trade_Request", GameLanguage.Trade_Request);
        GameLanguage.Item_CannotDrop = reader.ReadString("Language", "Item_CannotDrop", GameLanguage.Item_CannotDrop);
        GameLanguage.Creature_NameLengthInvalid = reader.ReadString("Language", "Creature_NameLengthInvalid", GameLanguage.Creature_NameLengthInvalid);
        GameLanguage.Creature_VerificationFailed = reader.ReadString("Language", "Creature_VerificationFailed", GameLanguage.Creature_VerificationFailed);
        GameLanguage.Creature_EnterName = reader.ReadString("Language", "Creature_EnterName", GameLanguage.Creature_EnterName);
        GameLanguage.Creature_EnterNewName = reader.ReadString("Language", "Creature_EnterNewName", GameLanguage.Creature_EnterNewName);
        GameLanguage.Creature_EnterNameForVerification = reader.ReadString("Language", "Creature_EnterNameForVerification", GameLanguage.Creature_EnterNameForVerification);
        GameLanguage.Keyboard_ResetDefault = reader.ReadString("Language", "Keyboard_ResetDefault", GameLanguage.Keyboard_ResetDefault);
        GameLanguage.Mail_DeleteParcelWithItemsConfirm = reader.ReadString("Language", "Mail_DeleteParcelWithItemsConfirm", GameLanguage.Mail_DeleteParcelWithItemsConfirm);
        GameLanguage.Mail_NoParcels = reader.ReadString("Language", "Mail_NoParcels", GameLanguage.Mail_NoParcels);
        GameLanguage.Mail_AllParcelsCollected = reader.ReadString("Language", "Mail_AllParcelsCollected", GameLanguage.Mail_AllParcelsCollected);
        GameLanguage.Reincarnation_Request = reader.ReadString("Language", "Reincarnation_Request", GameLanguage.Reincarnation_Request);
        GameLanguage.Potion_UseSpecialConfirm = reader.ReadString("Language", "Potion_UseSpecialConfirm", GameLanguage.Potion_UseSpecialConfirm);
        GameLanguage.Item_CombineConfirm = reader.ReadString("Language", "Item_CombineConfirm", GameLanguage.Item_CombineConfirm);
        GameLanguage.ItemRental_CancelledFaceOther = reader.ReadString("Language", "ItemRental_CancelledFaceOther", GameLanguage.ItemRental_CancelledFaceOther);
        GameLanguage.Item_SoulboundTo = reader.ReadString("Language", "Item_SoulboundTo", GameLanguage.Item_SoulboundTo);
        GameLanguage.Item_Cursed = reader.ReadString("Language", "Item_Cursed", GameLanguage.Item_Cursed);
        GameLanguage.Item_GemCannotUse = reader.ReadString("Language", "Item_GemCannotUse", GameLanguage.Item_GemCannotUse);
        GameLanguage.Item_GemCanUseOn = reader.ReadString("Language", "Item_GemCanUseOn", GameLanguage.Item_GemCanUseOn);
        GameLanguage.Item_GemUseOnWeapon = reader.ReadString("Language", "Item_GemUseOnWeapon", GameLanguage.Item_GemUseOnWeapon);
        GameLanguage.Item_GemUseOnArmour = reader.ReadString("Language", "Item_GemUseOnArmour", GameLanguage.Item_GemUseOnArmour);
        GameLanguage.Item_GemUseOnHelmet = reader.ReadString("Language", "Item_GemUseOnHelmet", GameLanguage.Item_GemUseOnHelmet);
        GameLanguage.Item_GemUseOnNecklace = reader.ReadString("Language", "Item_GemUseOnNecklace", GameLanguage.Item_GemUseOnNecklace);
        GameLanguage.Item_GemUseOnBracelet = reader.ReadString("Language", "Item_GemUseOnBracelet", GameLanguage.Item_GemUseOnBracelet);
        GameLanguage.Item_GemUseOnRing = reader.ReadString("Language", "Item_GemUseOnRing", GameLanguage.Item_GemUseOnRing);
        GameLanguage.Item_GemUseOnAmulet = reader.ReadString("Language", "Item_GemUseOnAmulet", GameLanguage.Item_GemUseOnAmulet);
        GameLanguage.Item_GemUseOnBelt = reader.ReadString("Language", "Item_GemUseOnBelt", GameLanguage.Item_GemUseOnBelt);
        GameLanguage.Item_GemUseOnBoots = reader.ReadString("Language", "Item_GemUseOnBoots", GameLanguage.Item_GemUseOnBoots);
        GameLanguage.Item_GemUseOnStone = reader.ReadString("Language", "Item_GemUseOnStone", GameLanguage.Item_GemUseOnStone);
        GameLanguage.Item_GemUseOnCandle = reader.ReadString("Language", "Item_GemUseOnCandle", GameLanguage.Item_GemUseOnCandle);
        GameLanguage.Item_SoulBindsOnEquip = reader.ReadString("Language", "Item_SoulBindsOnEquip", GameLanguage.Item_SoulBindsOnEquip);
        GameLanguage.Item_CannotBeUsedByHero = reader.ReadString("Language", "Item_CannotBeUsedByHero", GameLanguage.Item_CannotBeUsedByHero);
        GameLanguage.Item_ExpiresIn = reader.ReadString("Language", "Item_ExpiresIn", GameLanguage.Item_ExpiresIn);
        GameLanguage.Item_Expired = reader.ReadString("Language", "Item_Expired", GameLanguage.Item_Expired);
        GameLanguage.Item_SealedFor = reader.ReadString("Language", "Item_SealedFor", GameLanguage.Item_SealedFor);
        GameLanguage.Item_RentalFrom = reader.ReadString("Language", "Item_RentalFrom", GameLanguage.Item_RentalFrom);
        GameLanguage.Item_RentalExpiresIn = reader.ReadString("Language", "Item_RentalExpiresIn", GameLanguage.Item_RentalExpiresIn);
        GameLanguage.Item_RentalExpired = reader.ReadString("Language", "Item_RentalExpired", GameLanguage.Item_RentalExpired);
        GameLanguage.Item_RentalLockExpiresIn = reader.ReadString("Language", "Item_RentalLockExpiresIn", GameLanguage.Item_RentalLockExpiresIn);
        GameLanguage.Item_RentalLockExpired = reader.ReadString("Language", "Item_RentalLockExpired", GameLanguage.Item_RentalLockExpired);
        GameLanguage.Item_CantSpecialRepair = reader.ReadString("Language", "Item_CantSpecialRepair", GameLanguage.Item_CantSpecialRepair);
        GameLanguage.Item_BreaksOnDeath = reader.ReadString("Language", "Item_BreaksOnDeath", GameLanguage.Item_BreaksOnDeath);
        GameLanguage.Item_DestroyedWhenDropped = reader.ReadString("Language", "Item_DestroyedWhenDropped", GameLanguage.Item_DestroyedWhenDropped);
        GameLanguage.Item_CannotBeWeddingRing = reader.ReadString("Language", "Item_CannotBeWeddingRing", GameLanguage.Item_CannotBeWeddingRing);
        GameLanguage.Item_GemHint_RepairPartialWeaponAccessory = reader.ReadString("Language", "Item_GemHint_RepairPartialWeaponAccessory", GameLanguage.Item_GemHint_RepairPartialWeaponAccessory);
        GameLanguage.Item_GemHint_RepairPartialArmourDrapery = reader.ReadString("Language", "Item_GemHint_RepairPartialArmourDrapery", GameLanguage.Item_GemHint_RepairPartialArmourDrapery);
        GameLanguage.Item_GemHint_CombineMaybeDestroy = reader.ReadString("Language", "Item_GemHint_CombineMaybeDestroy", GameLanguage.Item_GemHint_CombineMaybeDestroy);
        GameLanguage.Item_GemHint_CombineNoDestroy = reader.ReadString("Language", "Item_GemHint_CombineNoDestroy", GameLanguage.Item_GemHint_CombineNoDestroy);
        GameLanguage.Item_GemHint_RepairFullWeaponAccessory = reader.ReadString("Language", "Item_GemHint_RepairFullWeaponAccessory", GameLanguage.Item_GemHint_RepairFullWeaponAccessory);
        GameLanguage.Item_GemHint_RepairFullArmourDrapery = reader.ReadString("Language", "Item_GemHint_RepairFullArmourDrapery", GameLanguage.Item_GemHint_RepairFullArmourDrapery);
        GameLanguage.Item_GemHint_SealItem = reader.ReadString("Language", "Item_GemHint_SealItem", GameLanguage.Item_GemHint_SealItem);
        GameLanguage.Item_CreditScroll_AddCredits = reader.ReadString("Language", "Item_CreditScroll_AddCredits", GameLanguage.Item_CreditScroll_AddCredits);
        GameLanguage.Item_CreatedByGameMaster = reader.ReadString("Language", "Item_CreatedByGameMaster", GameLanguage.Item_CreatedByGameMaster);
        GameLanguage.Mail_GoldLabel = reader.ReadString("Language", "Mail_GoldLabel", GameLanguage.Mail_GoldLabel);
        GameLanguage.Guild_JoinRequest = reader.ReadString("Language", "Guild_JoinRequest", GameLanguage.Guild_JoinRequest);
        GameLanguage.Guild_CreateEnterName = reader.ReadString("Language", "Guild_CreateEnterName", GameLanguage.Guild_CreateEnterName);
        GameLanguage.Guild_WarEnterName = reader.ReadString("Language", "Guild_WarEnterName", GameLanguage.Guild_WarEnterName);
        GameLanguage.Marriage_Request = reader.ReadString("Language", "Marriage_Request", GameLanguage.Marriage_Request);
        GameLanguage.Divorce_Request = reader.ReadString("Language", "Divorce_Request", GameLanguage.Divorce_Request);
        GameLanguage.Mentor_Request = reader.ReadString("Language", "Mentor_Request", GameLanguage.Mentor_Request);
        GameLanguage.Awakening_NotEnoughMaterials = reader.ReadString("Language", "Awakening_NotEnoughMaterials", GameLanguage.Awakening_NotEnoughMaterials);
        GameLanguage.Awakening_AlreadyMaxLevel = reader.ReadString("Language", "Awakening_AlreadyMaxLevel", GameLanguage.Awakening_AlreadyMaxLevel);
        GameLanguage.Awakening_CannotAwaken = reader.ReadString("Language", "Awakening_CannotAwaken", GameLanguage.Awakening_CannotAwaken);
        GameLanguage.BigMap_TeleportToNPC = reader.ReadString("Language", "BigMap_TeleportToNPC", GameLanguage.BigMap_TeleportToNPC);
        GameLanguage.BigMap_PathNotFound = reader.ReadString("Language", "BigMap_PathNotFound", GameLanguage.BigMap_PathNotFound);
        GameLanguage.BigMap_SearchForNPCs = reader.ReadString("Language", "BigMap_SearchForNPCs", GameLanguage.BigMap_SearchForNPCs);
        GameLanguage.System_PlayerNotOnline = reader.ReadString("Language", "System_PlayerNotOnline", GameLanguage.System_PlayerNotOnline);
        GameLanguage.TrustMerchant_GetBackUnsold = reader.ReadString("Language", "TrustMerchant_GetBackUnsold", GameLanguage.TrustMerchant_GetBackUnsold);
        GameLanguage.TrustMerchant_BuyConfirm = reader.ReadString("Language", "TrustMerchant_BuyConfirm", GameLanguage.TrustMerchant_BuyConfirm);
        GameLanguage.TrustMerchant_BidConfirm = reader.ReadString("Language", "TrustMerchant_BidConfirm", GameLanguage.TrustMerchant_BidConfirm);
        GameLanguage.TrustMerchant_SearchCooldown = reader.ReadString("Language", "TrustMerchant_SearchCooldown", GameLanguage.TrustMerchant_SearchCooldown);
        GameLanguage.TrustMerchant_FailDead = reader.ReadString("Language", "TrustMerchant_FailDead", GameLanguage.TrustMerchant_FailDead);
        GameLanguage.TrustMerchant_FailNotUsing = reader.ReadString("Language", "TrustMerchant_FailNotUsing", GameLanguage.TrustMerchant_FailNotUsing);
        GameLanguage.TrustMerchant_FailSold = reader.ReadString("Language", "TrustMerchant_FailSold", GameLanguage.TrustMerchant_FailSold);
        GameLanguage.TrustMerchant_FailExpired = reader.ReadString("Language", "TrustMerchant_FailExpired", GameLanguage.TrustMerchant_FailExpired);
        GameLanguage.TrustMerchant_FailWeightSpace = reader.ReadString("Language", "TrustMerchant_FailWeightSpace", GameLanguage.TrustMerchant_FailWeightSpace);
        GameLanguage.TrustMerchant_FailOwnItems = reader.ReadString("Language", "TrustMerchant_FailOwnItems", GameLanguage.TrustMerchant_FailOwnItems);
        GameLanguage.TrustMerchant_FailTooFar = reader.ReadString("Language", "TrustMerchant_FailTooFar", GameLanguage.TrustMerchant_FailTooFar);
        GameLanguage.TrustMerchant_FailHoldGold = reader.ReadString("Language", "TrustMerchant_FailHoldGold", GameLanguage.TrustMerchant_FailHoldGold);
        GameLanguage.TrustMerchant_FailMinBid = reader.ReadString("Language", "TrustMerchant_FailMinBid", GameLanguage.TrustMerchant_FailMinBid);
        GameLanguage.TrustMerchant_FailAuctionEnded = reader.ReadString("Language", "TrustMerchant_FailAuctionEnded", GameLanguage.TrustMerchant_FailAuctionEnded);
        GameLanguage.TrustMerchant_Title_SalePrice = reader.ReadString("Language", "TrustMerchant_Title_SalePrice", GameLanguage.TrustMerchant_Title_SalePrice);
        GameLanguage.TrustMerchant_Title_SellItem = reader.ReadString("Language", "TrustMerchant_Title_SellItem", GameLanguage.TrustMerchant_Title_SellItem);
        GameLanguage.TrustMerchant_Title_Item = reader.ReadString("Language", "TrustMerchant_Title_Item", GameLanguage.TrustMerchant_Title_Item);
        GameLanguage.TrustMerchant_Title_Price = reader.ReadString("Language", "TrustMerchant_Title_Price", GameLanguage.TrustMerchant_Title_Price);
        GameLanguage.TrustMerchant_Title_Expiry = reader.ReadString("Language", "TrustMerchant_Title_Expiry", GameLanguage.TrustMerchant_Title_Expiry);
        GameLanguage.TrustMerchant_Title_PriceBid = reader.ReadString("Language", "TrustMerchant_Title_PriceBid", GameLanguage.TrustMerchant_Title_PriceBid);
        GameLanguage.TrustMerchant_Title_SellerExpiry = reader.ReadString("Language", "TrustMerchant_Title_SellerExpiry", GameLanguage.TrustMerchant_Title_SellerExpiry);
        GameLanguage.TrustMerchant_Title_StartingBid = reader.ReadString("Language", "TrustMerchant_Title_StartingBid", GameLanguage.TrustMerchant_Title_StartingBid);
        GameLanguage.TrustMerchant_Title_HighestBid = reader.ReadString("Language", "TrustMerchant_Title_HighestBid", GameLanguage.TrustMerchant_Title_HighestBid);
        GameLanguage.TrustMerchant_Title_EndDate = reader.ReadString("Language", "TrustMerchant_Title_EndDate", GameLanguage.TrustMerchant_Title_EndDate);
        GameLanguage.TrustMerchant_ConsignHelp = reader.ReadString("Language", "TrustMerchant_ConsignHelp", GameLanguage.TrustMerchant_ConsignHelp);
        GameLanguage.TrustMerchant_AuctionHelp = reader.ReadString("Language", "TrustMerchant_AuctionHelp", GameLanguage.TrustMerchant_AuctionHelp);
        GameLanguage.TrustMerchant_Filter_All = reader.ReadString("Language", "TrustMerchant_Filter_All", GameLanguage.TrustMerchant_Filter_All);
        GameLanguage.TrustMerchant_Filter_Weapon = reader.ReadString("Language", "TrustMerchant_Filter_Weapon", GameLanguage.TrustMerchant_Filter_Weapon);
        GameLanguage.TrustMerchant_Filter_Drapery = reader.ReadString("Language", "TrustMerchant_Filter_Drapery", GameLanguage.TrustMerchant_Filter_Drapery);
        GameLanguage.TrustMerchant_Filter_Accessory = reader.ReadString("Language", "TrustMerchant_Filter_Accessory", GameLanguage.TrustMerchant_Filter_Accessory);
        GameLanguage.TrustMerchant_Filter_Consumable = reader.ReadString("Language", "TrustMerchant_Filter_Consumable", GameLanguage.TrustMerchant_Filter_Consumable);
        GameLanguage.TrustMerchant_Filter_Enhancement = reader.ReadString("Language", "TrustMerchant_Filter_Enhancement", GameLanguage.TrustMerchant_Filter_Enhancement);
        GameLanguage.TrustMerchant_Filter_Book = reader.ReadString("Language", "TrustMerchant_Filter_Book", GameLanguage.TrustMerchant_Filter_Book);
        GameLanguage.TrustMerchant_Filter_Craft = reader.ReadString("Language", "TrustMerchant_Filter_Craft", GameLanguage.TrustMerchant_Filter_Craft);
        GameLanguage.TrustMerchant_Filter_Drapery_Armour = reader.ReadString("Language", "TrustMerchant_Filter_Drapery_Armour", GameLanguage.TrustMerchant_Filter_Drapery_Armour);
        GameLanguage.TrustMerchant_Filter_Drapery_Helmet = reader.ReadString("Language", "TrustMerchant_Filter_Drapery_Helmet", GameLanguage.TrustMerchant_Filter_Drapery_Helmet);
        GameLanguage.TrustMerchant_Filter_Drapery_Belt = reader.ReadString("Language", "TrustMerchant_Filter_Drapery_Belt", GameLanguage.TrustMerchant_Filter_Drapery_Belt);
        GameLanguage.TrustMerchant_Filter_Drapery_Boots = reader.ReadString("Language", "TrustMerchant_Filter_Drapery_Boots", GameLanguage.TrustMerchant_Filter_Drapery_Boots);
        GameLanguage.TrustMerchant_Filter_Drapery_Stone = reader.ReadString("Language", "TrustMerchant_Filter_Drapery_Stone", GameLanguage.TrustMerchant_Filter_Drapery_Stone);
        GameLanguage.TrustMerchant_Filter_Accessory_Necklaces = reader.ReadString("Language", "TrustMerchant_Filter_Accessory_Necklaces", GameLanguage.TrustMerchant_Filter_Accessory_Necklaces);
        GameLanguage.TrustMerchant_Filter_Accessory_Bracelets = reader.ReadString("Language", "TrustMerchant_Filter_Accessory_Bracelets", GameLanguage.TrustMerchant_Filter_Accessory_Bracelets);
        GameLanguage.TrustMerchant_Filter_Accessory_Rings = reader.ReadString("Language", "TrustMerchant_Filter_Accessory_Rings", GameLanguage.TrustMerchant_Filter_Accessory_Rings);
        GameLanguage.TrustMerchant_Filter_Consumable_Recovery = reader.ReadString("Language", "TrustMerchant_Filter_Consumable_Recovery", GameLanguage.TrustMerchant_Filter_Consumable_Recovery);
        GameLanguage.TrustMerchant_Filter_Consumable_Buff = reader.ReadString("Language", "TrustMerchant_Filter_Consumable_Buff", GameLanguage.TrustMerchant_Filter_Consumable_Buff);
        GameLanguage.TrustMerchant_Filter_Consumable_Scrolls = reader.ReadString("Language", "TrustMerchant_Filter_Consumable_Scrolls", GameLanguage.TrustMerchant_Filter_Consumable_Scrolls);
        GameLanguage.TrustMerchant_Filter_Consumable_Misc = reader.ReadString("Language", "TrustMerchant_Filter_Consumable_Misc", GameLanguage.TrustMerchant_Filter_Consumable_Misc);
        GameLanguage.TrustMerchant_Filter_Enhancement_Gems = reader.ReadString("Language", "TrustMerchant_Filter_Enhancement_Gems", GameLanguage.TrustMerchant_Filter_Enhancement_Gems);
        GameLanguage.TrustMerchant_Filter_Enhancement_Orbs = reader.ReadString("Language", "TrustMerchant_Filter_Enhancement_Orbs", GameLanguage.TrustMerchant_Filter_Enhancement_Orbs);
        GameLanguage.TrustMerchant_Filter_Book_Warrior = reader.ReadString("Language", "TrustMerchant_Filter_Book_Warrior", GameLanguage.TrustMerchant_Filter_Book_Warrior);
        GameLanguage.TrustMerchant_Filter_Book_Wizard = reader.ReadString("Language", "TrustMerchant_Filter_Book_Wizard", GameLanguage.TrustMerchant_Filter_Book_Wizard);
        GameLanguage.TrustMerchant_Filter_Book_Taoist = reader.ReadString("Language", "TrustMerchant_Filter_Book_Taoist", GameLanguage.TrustMerchant_Filter_Book_Taoist);
        GameLanguage.TrustMerchant_Filter_Book_Assassin = reader.ReadString("Language", "TrustMerchant_Filter_Book_Assassin", GameLanguage.TrustMerchant_Filter_Book_Assassin);
        GameLanguage.TrustMerchant_Filter_Book_Archer = reader.ReadString("Language", "TrustMerchant_Filter_Book_Archer", GameLanguage.TrustMerchant_Filter_Book_Archer);
        GameLanguage.TrustMerchant_Filter_Craft_Materials = reader.ReadString("Language", "TrustMerchant_Filter_Craft_Materials", GameLanguage.TrustMerchant_Filter_Craft_Materials);
        GameLanguage.TrustMerchant_Filter_Craft_Meat = reader.ReadString("Language", "TrustMerchant_Filter_Craft_Meat", GameLanguage.TrustMerchant_Filter_Craft_Meat);
        GameLanguage.TrustMerchant_Filter_Craft_Ore = reader.ReadString("Language", "TrustMerchant_Filter_Craft_Ore", GameLanguage.TrustMerchant_Filter_Craft_Ore);
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
        GameLanguage.Guild_MemberLoggedOn = reader.ReadString("Language", "Guild_MemberLoggedOn", GameLanguage.Guild_MemberLoggedOn);
        GameLanguage.Guild_MemberJoined = reader.ReadString("Language", "Guild_MemberJoined", GameLanguage.Guild_MemberJoined);
        GameLanguage.Guild_MemberKicked = reader.ReadString("Language", "Guild_MemberKicked", GameLanguage.Guild_MemberKicked);
        GameLanguage.Guild_MemberLeft = reader.ReadString("Language", "Guild_MemberLeft", GameLanguage.Guild_MemberLeft);
        GameLanguage.Guild_DonatedToFund = reader.ReadString("Language", "Guild_DonatedToFund", GameLanguage.Guild_DonatedToFund);
        GameLanguage.Guild_RetrievedFromFund = reader.ReadString("Language", "Guild_RetrievedFromFund", GameLanguage.Guild_RetrievedFromFund);

        GameLanguage.Ranking_OnlineOnly   = reader.ReadString("Language", "Ranking_OnlineOnly",   GameLanguage.Ranking_OnlineOnly);
        GameLanguage.Ranking_AllHint      = reader.ReadString("Language", "Ranking_AllHint",      GameLanguage.Ranking_AllHint);
        GameLanguage.Ranking_WarriorHint  = reader.ReadString("Language", "Ranking_WarriorHint",  GameLanguage.Ranking_WarriorHint);
        GameLanguage.Ranking_WizardHint   = reader.ReadString("Language", "Ranking_WizardHint",   GameLanguage.Ranking_WizardHint);
        GameLanguage.Ranking_TaoistHint   = reader.ReadString("Language", "Ranking_TaoistHint",   GameLanguage.Ranking_TaoistHint);
        GameLanguage.Ranking_AssassinHint = reader.ReadString("Language", "Ranking_AssassinHint", GameLanguage.Ranking_AssassinHint);
        GameLanguage.Ranking_ArcherHint   = reader.ReadString("Language", "Ranking_ArcherHint",   GameLanguage.Ranking_ArcherHint);
        GameLanguage.Ranking_NotListed    = reader.ReadString("Language", "Ranking_NotListed",    GameLanguage.Ranking_NotListed);
        GameLanguage.Ranking_Ranked       = reader.ReadString("Language", "Ranking_Ranked",       GameLanguage.Ranking_Ranked);

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
        GameLanguage.Group_MaxMembers = reader.ReadString("Language", "Group_MaxMembers", GameLanguage.Group_MaxMembers);
        GameLanguage.Group_NotLeader = reader.ReadString("Language", "Group_NotLeader", GameLanguage.Group_NotLeader);
        GameLanguage.Guild_InvalidNameBackslash = reader.ReadString("Language", "Guild_InvalidNameBackslash", GameLanguage.Guild_InvalidNameBackslash);
        GameLanguage.TooHeavyToHold = reader.ReadString("Language", "TooHeavyToHold", GameLanguage.TooHeavyToHold);
        GameLanguage.TooHeavyToWear = reader.ReadString("Language", "TooHeavyToWear", GameLanguage.TooHeavyToWear);
        GameLanguage.Chat_ItemLinkTooLong = reader.ReadString("Language", "Chat_ItemLinkTooLong", GameLanguage.Chat_ItemLinkTooLong);
        GameLanguage.Item_NoRoomToSplitStack = reader.ReadString("Language", "Item_NoRoomToSplitStack", GameLanguage.Item_NoRoomToSplitStack);
        GameLanguage.Item_CannotSwapItems = reader.ReadString("Language", "Item_CannotSwapItems", GameLanguage.Item_CannotSwapItems);
        GameLanguage.Guild_InsufficientRetrieveItems = reader.ReadString("Language", "Guild_InsufficientRetrieveItems", GameLanguage.Guild_InsufficientRetrieveItems);
        GameLanguage.Guild_InsufficientStoreItems = reader.ReadString("Language", "Guild_InsufficientStoreItems", GameLanguage.Guild_InsufficientStoreItems);
        GameLanguage.Rental_CannotRemoveLockedItem = reader.ReadString("Language", "Rental_CannotRemoveLockedItem", GameLanguage.Rental_CannotRemoveLockedItem);
        GameLanguage.Mail_CannotMailItem = reader.ReadString("Language", "Mail_CannotMailItem", GameLanguage.Mail_CannotMailItem);
        GameLanguage.Item_TooHeavyToTransfer = reader.ReadString("Language", "Item_TooHeavyToTransfer", GameLanguage.Item_TooHeavyToTransfer);
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
        GameLanguage.System_TargetTooFar = reader.ReadString("Language", "System_TargetTooFar", GameLanguage.System_TargetTooFar);
        GameLanguage.System_ObservationDisabled = reader.ReadString("Language", "System_ObservationDisabled", GameLanguage.System_ObservationDisabled);

        // Group related
        GameLanguage.Group_YouLeft = reader.ReadString("Language", "Group_YouLeft", GameLanguage.Group_YouLeft);
        GameLanguage.Group_PlayerLeft = reader.ReadString("Language", "Group_PlayerLeft", GameLanguage.Group_PlayerLeft);
        GameLanguage.Group_PlayerJoined = reader.ReadString("Language", "Group_PlayerJoined", GameLanguage.Group_PlayerJoined);
        GameLanguage.Group_InviteQuestion = reader.ReadString("Language", "Group_InviteQuestion", GameLanguage.Group_InviteQuestion);

        // Quest related
        GameLanguage.Quest_ShareQuestion = reader.ReadString("Language", "Quest_ShareQuestion", GameLanguage.Quest_ShareQuestion);
        GameLanguage.Quest_TaskTitle = reader.ReadString("Language", "Quest_TaskTitle", GameLanguage.Quest_TaskTitle);
        GameLanguage.Quest_ProgressTitle = reader.ReadString("Language", "Quest_ProgressTitle", GameLanguage.Quest_ProgressTitle);
        GameLanguage.Quest_ReturnTitle = reader.ReadString("Language", "Quest_ReturnTitle", GameLanguage.Quest_ReturnTitle);
        GameLanguage.Quest_TimeLimitTitle = reader.ReadString("Language", "Quest_TimeLimitTitle", GameLanguage.Quest_TimeLimitTitle);
        GameLanguage.Quest_SelectRewardRequired = reader.ReadString("Language", "Quest_SelectRewardRequired", GameLanguage.Quest_SelectRewardRequired);
        GameLanguage.Quest_CancelConfirm = reader.ReadString("Language", "Quest_CancelConfirm", GameLanguage.Quest_CancelConfirm);

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
        GameLanguage.Item_WarriorCannotUse = reader.ReadString("Language", "Item_WarriorCannotUse", GameLanguage.Item_WarriorCannotUse);
        GameLanguage.Item_WizardCannotUse = reader.ReadString("Language", "Item_WizardCannotUse", GameLanguage.Item_WizardCannotUse);
        GameLanguage.Item_TaoistCannotUse = reader.ReadString("Language", "Item_TaoistCannotUse", GameLanguage.Item_TaoistCannotUse);
        GameLanguage.Item_AssassinCannotUse = reader.ReadString("Language", "Item_AssassinCannotUse", GameLanguage.Item_AssassinCannotUse);
        GameLanguage.Item_ArcherCannotUse = reader.ReadString("Language", "Item_ArcherCannotUse", GameLanguage.Item_ArcherCannotUse);
        GameLanguage.Req_NotEnoughAC = reader.ReadString("Language", "Req_NotEnoughAC", GameLanguage.Req_NotEnoughAC);
        GameLanguage.Req_NotEnoughMAC = reader.ReadString("Language", "Req_NotEnoughMAC", GameLanguage.Req_NotEnoughMAC);
        GameLanguage.Req_MaxLevelExceeded = reader.ReadString("Language", "Req_MaxLevelExceeded", GameLanguage.Req_MaxLevelExceeded);
        GameLanguage.Req_NotEnoughBaseAC = reader.ReadString("Language", "Req_NotEnoughBaseAC", GameLanguage.Req_NotEnoughBaseAC);
        GameLanguage.Req_NotEnoughBaseMAC = reader.ReadString("Language", "Req_NotEnoughBaseMAC", GameLanguage.Req_NotEnoughBaseMAC);
        GameLanguage.Req_NotEnoughBaseDC = reader.ReadString("Language", "Req_NotEnoughBaseDC", GameLanguage.Req_NotEnoughBaseDC);
        GameLanguage.Req_NotEnoughBaseMC = reader.ReadString("Language", "Req_NotEnoughBaseMC", GameLanguage.Req_NotEnoughBaseMC);
        GameLanguage.Req_NotEnoughBaseSC = reader.ReadString("Language", "Req_NotEnoughBaseSC", GameLanguage.Req_NotEnoughBaseSC);
        GameLanguage.Req_NoMountEquipped = reader.ReadString("Language", "Req_NoMountEquipped", GameLanguage.Req_NoMountEquipped);
        GameLanguage.Req_NoFishingRodEquipped = reader.ReadString("Language", "Req_NoFishingRodEquipped", GameLanguage.Req_NoFishingRodEquipped);

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
        reader.Write("Language", "MiniMap_Hint", GameLanguage.MiniMap_Hint);
        reader.Write("Language", "Inspect_InviteToGroup", GameLanguage.Inspect_InviteToGroup);
        reader.Write("Language", "Inspect_AddToFriends", GameLanguage.Inspect_AddToFriends);
        reader.Write("Language", "Inspect_SendMail", GameLanguage.Inspect_SendMail);
        reader.Write("Language", "Inspect_Trade", GameLanguage.Inspect_Trade);
        reader.Write("Language", "Inspect_Observe", GameLanguage.Inspect_Observe);
        reader.Write("Language", "GuildTerritory_None", GameLanguage.GuildTerritory_None);
        reader.Write("Language", "GuildTerritory_StatusAvailable", GameLanguage.GuildTerritory_StatusAvailable);
        reader.Write("Language", "GuildTerritory_StatusForSale", GameLanguage.GuildTerritory_StatusForSale);
        reader.Write("Language", "GuildTerritory_StatusSalePending", GameLanguage.GuildTerritory_StatusSalePending);
        reader.Write("Language", "GuildTerritory_StatusUnavailable", GameLanguage.GuildTerritory_StatusUnavailable);
        reader.Write("Language", "GuildTerritory_OwnerAndPrefix", GameLanguage.GuildTerritory_OwnerAndPrefix);
        reader.Write("Language", "Mail_ReportBug", GameLanguage.Mail_ReportBug);
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
        reader.Write("Language", "GameName", GameLanguage.GameName);
        reader.Write("Language", "ClassName_Warrior", GameLanguage.ClassName_Warrior);
        reader.Write("Language", "ClassName_Wizard", GameLanguage.ClassName_Wizard);
        reader.Write("Language", "ClassName_Taoist", GameLanguage.ClassName_Taoist);
        reader.Write("Language", "ClassName_Assassin", GameLanguage.ClassName_Assassin);
        reader.Write("Language", "ClassName_Archer", GameLanguage.ClassName_Archer);

        reader.Write("Language", "LowLevel", GameLanguage.LowLevel);
        reader.Write("Language", "LowGold", GameLanguage.LowGold);
        reader.Write("Language", "LowDC", GameLanguage.LowDC);
        reader.Write("Language", "LowMC", GameLanguage.LowMC);
        reader.Write("Language", "LowSC", GameLanguage.LowSC);
        reader.Write("Language", "NPC_NotEnoughPearls", GameLanguage.NPC_NotEnoughPearls);
        reader.Write("Language", "NPC_CannotSellItem", GameLanguage.NPC_CannotSellItem);
        reader.Write("Language", "NPC_CannotCarryMoreGold", GameLanguage.NPC_CannotCarryMoreGold);
        reader.Write("Language", "NPC_CannotRepairItem", GameLanguage.NPC_CannotRepairItem);
        reader.Write("Language", "NPC_CannotConsignItem", GameLanguage.NPC_CannotConsignItem);
        reader.Write("Language", "NPC_NotEnoughGold", GameLanguage.NPC_NotEnoughGold);
        reader.Write("Language", "NPC_MissingToolsOrIngredients", GameLanguage.NPC_MissingToolsOrIngredients);
        reader.Write("Language", "NPC_SellPrefix", GameLanguage.NPC_SellPrefix);
        reader.Write("Language", "NPC_RepairPrefix", GameLanguage.NPC_RepairPrefix);
        reader.Write("Language", "NPC_SpecialRepairPrefix", GameLanguage.NPC_SpecialRepairPrefix);
        reader.Write("Language", "NPC_ConsignPrefix", GameLanguage.NPC_ConsignPrefix);
        reader.Write("Language", "NPC_DisassembleWarning", GameLanguage.NPC_DisassembleWarning);
        reader.Write("Language", "NPC_DowngradePrefix", GameLanguage.NPC_DowngradePrefix);
        reader.Write("Language", "NPC_ResetPrefix", GameLanguage.NPC_ResetPrefix);
        reader.Write("Language", "NPC_RefinePrefix", GameLanguage.NPC_RefinePrefix);
        reader.Write("Language", "NPC_CheckRefine", GameLanguage.NPC_CheckRefine);
        reader.Write("Language", "NPC_ReplaceWedRingPrefix", GameLanguage.NPC_ReplaceWedRingPrefix);

        reader.Write("Language", "Gold", GameLanguage.Gold);
        reader.Write("Language", "Credit", GameLanguage.Credit);

        reader.Write("Language", "GameShop_BuyWithGold", GameLanguage.GameShop_BuyWithGold);
        reader.Write("Language", "GameShop_BuyWithCredits", GameLanguage.GameShop_BuyWithCredits);
        reader.Write("Language", "GameShop_BuyWithGoldHint", GameLanguage.GameShop_BuyWithGoldHint);
        reader.Write("Language", "GameShop_BuyWithCreditsHint", GameLanguage.GameShop_BuyWithCreditsHint);
        reader.Write("Language", "GameShop_ShowAll", GameLanguage.GameShop_ShowAll);
        reader.Write("Language", "GameShop_PageText", GameLanguage.GameShop_PageText);
        reader.Write("Language", "GameShop_BuyConfirmCredits", GameLanguage.GameShop_BuyConfirmCredits);
        reader.Write("Language", "GameShop_BuyConfirmGold", GameLanguage.GameShop_BuyConfirmGold);
        reader.Write("Language", "GameShop_SelectPaymentTypeRequired", GameLanguage.GameShop_SelectPaymentTypeRequired);
        reader.Write("Language", "GameShop_CannotAffordItem", GameLanguage.GameShop_CannotAffordItem);

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
        reader.Write("Language", "AttemptingConnectFirst", GameLanguage.AttemptingConnectFirst);
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
        reader.Write("Language", "Login_SendingClientVersion", GameLanguage.Login_SendingClientVersion);
        reader.Write("Language", "Login_DescAccountID", GameLanguage.Login_DescAccountID);
        reader.Write("Language", "Login_DescPassword", GameLanguage.Login_DescPassword);
        reader.Write("Language", "Login_DescEMail", GameLanguage.Login_DescEMail);
        reader.Write("Language", "Login_DescUserName", GameLanguage.Login_DescUserName);
        reader.Write("Language", "Login_DescBirthDate", GameLanguage.Login_DescBirthDate);
        reader.Write("Language", "Login_DescQuestion", GameLanguage.Login_DescQuestion);
        reader.Write("Language", "Login_DescAnswer", GameLanguage.Login_DescAnswer);

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
        reader.Write("Language", "Mail_TitleType", GameLanguage.Mail_TitleType);
        reader.Write("Language", "Mail_TitleSender", GameLanguage.Mail_TitleSender);
        reader.Write("Language", "Mail_TitleMessage", GameLanguage.Mail_TitleMessage);
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
        reader.Write("Language", "Friend_RemoveConfirm", GameLanguage.Friend_RemoveConfirm);
        reader.Write("Language", "Trade_DealCancelledFaceOther", GameLanguage.Trade_DealCancelledFaceOther);
        reader.Write("Language", "Trade_Request", GameLanguage.Trade_Request);
        reader.Write("Language", "Item_CannotDrop", GameLanguage.Item_CannotDrop);
        reader.Write("Language", "Creature_NameLengthInvalid", GameLanguage.Creature_NameLengthInvalid);
        reader.Write("Language", "Creature_EnterName", GameLanguage.Creature_EnterName);
        reader.Write("Language", "Creature_EnterNewName", GameLanguage.Creature_EnterNewName);
        reader.Write("Language", "Creature_EnterNameForVerification", GameLanguage.Creature_EnterNameForVerification);
        reader.Write("Language", "Keyboard_ResetDefault", GameLanguage.Keyboard_ResetDefault);
        reader.Write("Language", "Mail_DeleteParcelWithItemsConfirm", GameLanguage.Mail_DeleteParcelWithItemsConfirm);
        reader.Write("Language", "Mail_NoParcels", GameLanguage.Mail_NoParcels);
        reader.Write("Language", "Mail_AllParcelsCollected", GameLanguage.Mail_AllParcelsCollected);
        reader.Write("Language", "Reincarnation_Request", GameLanguage.Reincarnation_Request);
        reader.Write("Language", "Potion_UseSpecialConfirm", GameLanguage.Potion_UseSpecialConfirm);
        reader.Write("Language", "Item_CombineConfirm", GameLanguage.Item_CombineConfirm);
        reader.Write("Language", "ItemRental_CancelledFaceOther", GameLanguage.ItemRental_CancelledFaceOther);
        reader.Write("Language", "Item_SoulboundTo", GameLanguage.Item_SoulboundTo);
        reader.Write("Language", "Item_Cursed", GameLanguage.Item_Cursed);
        reader.Write("Language", "Item_GemCannotUse", GameLanguage.Item_GemCannotUse);
        reader.Write("Language", "Item_GemCanUseOn", GameLanguage.Item_GemCanUseOn);
        reader.Write("Language", "Item_GemUseOnWeapon", GameLanguage.Item_GemUseOnWeapon);
        reader.Write("Language", "Item_GemUseOnArmour", GameLanguage.Item_GemUseOnArmour);
        reader.Write("Language", "Item_GemUseOnHelmet", GameLanguage.Item_GemUseOnHelmet);
        reader.Write("Language", "Item_GemUseOnNecklace", GameLanguage.Item_GemUseOnNecklace);
        reader.Write("Language", "Item_GemUseOnBracelet", GameLanguage.Item_GemUseOnBracelet);
        reader.Write("Language", "Item_GemUseOnRing", GameLanguage.Item_GemUseOnRing);
        reader.Write("Language", "Item_GemUseOnAmulet", GameLanguage.Item_GemUseOnAmulet);
        reader.Write("Language", "Item_GemUseOnBelt", GameLanguage.Item_GemUseOnBelt);
        reader.Write("Language", "Item_GemUseOnBoots", GameLanguage.Item_GemUseOnBoots);
        reader.Write("Language", "Item_GemUseOnStone", GameLanguage.Item_GemUseOnStone);
        reader.Write("Language", "Item_GemUseOnCandle", GameLanguage.Item_GemUseOnCandle);
        reader.Write("Language", "Item_SoulBindsOnEquip", GameLanguage.Item_SoulBindsOnEquip);
        reader.Write("Language", "Item_CannotBeUsedByHero", GameLanguage.Item_CannotBeUsedByHero);
        reader.Write("Language", "Item_ExpiresIn", GameLanguage.Item_ExpiresIn);
        reader.Write("Language", "Item_Expired", GameLanguage.Item_Expired);
        reader.Write("Language", "Item_SealedFor", GameLanguage.Item_SealedFor);
        reader.Write("Language", "Item_RentalFrom", GameLanguage.Item_RentalFrom);
        reader.Write("Language", "Item_RentalExpiresIn", GameLanguage.Item_RentalExpiresIn);
        reader.Write("Language", "Item_RentalExpired", GameLanguage.Item_RentalExpired);
        reader.Write("Language", "Item_RentalLockExpiresIn", GameLanguage.Item_RentalLockExpiresIn);
        reader.Write("Language", "Item_RentalLockExpired", GameLanguage.Item_RentalLockExpired);
        reader.Write("Language", "Item_CantSpecialRepair", GameLanguage.Item_CantSpecialRepair);
        reader.Write("Language", "Item_BreaksOnDeath", GameLanguage.Item_BreaksOnDeath);
        reader.Write("Language", "Item_DestroyedWhenDropped", GameLanguage.Item_DestroyedWhenDropped);
        reader.Write("Language", "Item_CannotBeWeddingRing", GameLanguage.Item_CannotBeWeddingRing);
        reader.Write("Language", "Item_GemHint_RepairPartialWeaponAccessory", GameLanguage.Item_GemHint_RepairPartialWeaponAccessory);
        reader.Write("Language", "Item_GemHint_RepairPartialArmourDrapery", GameLanguage.Item_GemHint_RepairPartialArmourDrapery);
        reader.Write("Language", "Item_GemHint_CombineMaybeDestroy", GameLanguage.Item_GemHint_CombineMaybeDestroy);
        reader.Write("Language", "Item_GemHint_CombineNoDestroy", GameLanguage.Item_GemHint_CombineNoDestroy);
        reader.Write("Language", "Item_GemHint_RepairFullWeaponAccessory", GameLanguage.Item_GemHint_RepairFullWeaponAccessory);
        reader.Write("Language", "Item_GemHint_RepairFullArmourDrapery", GameLanguage.Item_GemHint_RepairFullArmourDrapery);
        reader.Write("Language", "Item_GemHint_SealItem", GameLanguage.Item_GemHint_SealItem);
        reader.Write("Language", "Item_CreditScroll_AddCredits", GameLanguage.Item_CreditScroll_AddCredits);
        reader.Write("Language", "Item_CreatedByGameMaster", GameLanguage.Item_CreatedByGameMaster);
        reader.Write("Language", "Mail_GoldLabel", GameLanguage.Mail_GoldLabel);
        reader.Write("Language", "Guild_JoinRequest", GameLanguage.Guild_JoinRequest);
        reader.Write("Language", "Guild_CreateEnterName", GameLanguage.Guild_CreateEnterName);
        reader.Write("Language", "Guild_WarEnterName", GameLanguage.Guild_WarEnterName);
        reader.Write("Language", "Marriage_Request", GameLanguage.Marriage_Request);
        reader.Write("Language", "Divorce_Request", GameLanguage.Divorce_Request);
        reader.Write("Language", "Mentor_Request", GameLanguage.Mentor_Request);
        reader.Write("Language", "Awakening_NotEnoughMaterials", GameLanguage.Awakening_NotEnoughMaterials);
        reader.Write("Language", "Awakening_AlreadyMaxLevel", GameLanguage.Awakening_AlreadyMaxLevel);
        reader.Write("Language", "Awakening_CannotAwaken", GameLanguage.Awakening_CannotAwaken);
        reader.Write("Language", "BigMap_TeleportToNPC", GameLanguage.BigMap_TeleportToNPC);
        reader.Write("Language", "BigMap_SearchForNPCs", GameLanguage.BigMap_SearchForNPCs);
        reader.Write("Language", "System_PlayerNotOnline", GameLanguage.System_PlayerNotOnline);
        reader.Write("Language", "TrustMerchant_GetBackUnsold", GameLanguage.TrustMerchant_GetBackUnsold);
        reader.Write("Language", "TrustMerchant_BuyConfirm", GameLanguage.TrustMerchant_BuyConfirm);
        reader.Write("Language", "TrustMerchant_BidConfirm", GameLanguage.TrustMerchant_BidConfirm);
        reader.Write("Language", "TrustMerchant_SearchCooldown", GameLanguage.TrustMerchant_SearchCooldown);
        reader.Write("Language", "TrustMerchant_Title_SalePrice", GameLanguage.TrustMerchant_Title_SalePrice);
        reader.Write("Language", "TrustMerchant_Title_SellItem", GameLanguage.TrustMerchant_Title_SellItem);
        reader.Write("Language", "TrustMerchant_Title_Item", GameLanguage.TrustMerchant_Title_Item);
        reader.Write("Language", "TrustMerchant_Title_Price", GameLanguage.TrustMerchant_Title_Price);
        reader.Write("Language", "TrustMerchant_Title_Expiry", GameLanguage.TrustMerchant_Title_Expiry);
        reader.Write("Language", "TrustMerchant_Title_PriceBid", GameLanguage.TrustMerchant_Title_PriceBid);
        reader.Write("Language", "TrustMerchant_Title_SellerExpiry", GameLanguage.TrustMerchant_Title_SellerExpiry);
        reader.Write("Language", "TrustMerchant_Title_StartingBid", GameLanguage.TrustMerchant_Title_StartingBid);
        reader.Write("Language", "TrustMerchant_Title_HighestBid", GameLanguage.TrustMerchant_Title_HighestBid);
        reader.Write("Language", "TrustMerchant_Title_EndDate", GameLanguage.TrustMerchant_Title_EndDate);
        reader.Write("Language", "TrustMerchant_ConsignHelp", GameLanguage.TrustMerchant_ConsignHelp);
        reader.Write("Language", "TrustMerchant_AuctionHelp", GameLanguage.TrustMerchant_AuctionHelp);
        reader.Write("Language", "TrustMerchant_Filter_All", GameLanguage.TrustMerchant_Filter_All);
        reader.Write("Language", "TrustMerchant_Filter_Weapon", GameLanguage.TrustMerchant_Filter_Weapon);
        reader.Write("Language", "TrustMerchant_Filter_Drapery", GameLanguage.TrustMerchant_Filter_Drapery);
        reader.Write("Language", "TrustMerchant_Filter_Accessory", GameLanguage.TrustMerchant_Filter_Accessory);
        reader.Write("Language", "TrustMerchant_Filter_Consumable", GameLanguage.TrustMerchant_Filter_Consumable);
        reader.Write("Language", "TrustMerchant_Filter_Enhancement", GameLanguage.TrustMerchant_Filter_Enhancement);
        reader.Write("Language", "TrustMerchant_Filter_Book", GameLanguage.TrustMerchant_Filter_Book);
        reader.Write("Language", "TrustMerchant_Filter_Craft", GameLanguage.TrustMerchant_Filter_Craft);
        reader.Write("Language", "TrustMerchant_Filter_Drapery_Armour", GameLanguage.TrustMerchant_Filter_Drapery_Armour);
        reader.Write("Language", "TrustMerchant_Filter_Drapery_Helmet", GameLanguage.TrustMerchant_Filter_Drapery_Helmet);
        reader.Write("Language", "TrustMerchant_Filter_Drapery_Belt", GameLanguage.TrustMerchant_Filter_Drapery_Belt);
        reader.Write("Language", "TrustMerchant_Filter_Drapery_Boots", GameLanguage.TrustMerchant_Filter_Drapery_Boots);
        reader.Write("Language", "TrustMerchant_Filter_Drapery_Stone", GameLanguage.TrustMerchant_Filter_Drapery_Stone);
        reader.Write("Language", "TrustMerchant_Filter_Accessory_Necklaces", GameLanguage.TrustMerchant_Filter_Accessory_Necklaces);
        reader.Write("Language", "TrustMerchant_Filter_Accessory_Bracelets", GameLanguage.TrustMerchant_Filter_Accessory_Bracelets);
        reader.Write("Language", "TrustMerchant_Filter_Accessory_Rings", GameLanguage.TrustMerchant_Filter_Accessory_Rings);
        reader.Write("Language", "TrustMerchant_Filter_Consumable_Recovery", GameLanguage.TrustMerchant_Filter_Consumable_Recovery);
        reader.Write("Language", "TrustMerchant_Filter_Consumable_Buff", GameLanguage.TrustMerchant_Filter_Consumable_Buff);
        reader.Write("Language", "TrustMerchant_Filter_Consumable_Scrolls", GameLanguage.TrustMerchant_Filter_Consumable_Scrolls);
        reader.Write("Language", "TrustMerchant_Filter_Consumable_Misc", GameLanguage.TrustMerchant_Filter_Consumable_Misc);
        reader.Write("Language", "TrustMerchant_Filter_Enhancement_Gems", GameLanguage.TrustMerchant_Filter_Enhancement_Gems);
        reader.Write("Language", "TrustMerchant_Filter_Enhancement_Orbs", GameLanguage.TrustMerchant_Filter_Enhancement_Orbs);
        reader.Write("Language", "TrustMerchant_Filter_Book_Warrior", GameLanguage.TrustMerchant_Filter_Book_Warrior);
        reader.Write("Language", "TrustMerchant_Filter_Book_Wizard", GameLanguage.TrustMerchant_Filter_Book_Wizard);
        reader.Write("Language", "TrustMerchant_Filter_Book_Taoist", GameLanguage.TrustMerchant_Filter_Book_Taoist);
        reader.Write("Language", "TrustMerchant_Filter_Book_Assassin", GameLanguage.TrustMerchant_Filter_Book_Assassin);
        reader.Write("Language", "TrustMerchant_Filter_Book_Archer", GameLanguage.TrustMerchant_Filter_Book_Archer);
        reader.Write("Language", "TrustMerchant_Filter_Craft_Materials", GameLanguage.TrustMerchant_Filter_Craft_Materials);
        reader.Write("Language", "TrustMerchant_Filter_Craft_Meat", GameLanguage.TrustMerchant_Filter_Craft_Meat);
        reader.Write("Language", "TrustMerchant_Filter_Craft_Ore", GameLanguage.TrustMerchant_Filter_Craft_Ore);
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
        reader.Write("Language", "Guild_MemberLoggedOn", GameLanguage.Guild_MemberLoggedOn);
        reader.Write("Language", "Guild_MemberJoined", GameLanguage.Guild_MemberJoined);
        reader.Write("Language", "Guild_MemberKicked", GameLanguage.Guild_MemberKicked);
        reader.Write("Language", "Guild_MemberLeft", GameLanguage.Guild_MemberLeft);
        reader.Write("Language", "Guild_DonatedToFund", GameLanguage.Guild_DonatedToFund);
        reader.Write("Language", "Guild_RetrievedFromFund", GameLanguage.Guild_RetrievedFromFund);

        reader.Write("Language", "Ranking_OnlineOnly",   GameLanguage.Ranking_OnlineOnly);
        reader.Write("Language", "Ranking_AllHint",      GameLanguage.Ranking_AllHint);
        reader.Write("Language", "Ranking_WarriorHint",  GameLanguage.Ranking_WarriorHint);
        reader.Write("Language", "Ranking_WizardHint",   GameLanguage.Ranking_WizardHint);
        reader.Write("Language", "Ranking_TaoistHint",   GameLanguage.Ranking_TaoistHint);
        reader.Write("Language", "Ranking_AssassinHint", GameLanguage.Ranking_AssassinHint);
        reader.Write("Language", "Ranking_ArcherHint",   GameLanguage.Ranking_ArcherHint);
        reader.Write("Language", "Ranking_NotListed",    GameLanguage.Ranking_NotListed);
        reader.Write("Language", "Ranking_Ranked",       GameLanguage.Ranking_Ranked);

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
        reader.Write("Language", "TooHeavyToWear", GameLanguage.TooHeavyToWear);
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
        reader.Write("Language", "System_TargetTooFar", GameLanguage.System_TargetTooFar);

        // Group related
        reader.Write("Language", "Group_YouLeft", GameLanguage.Group_YouLeft);
        reader.Write("Language", "Group_PlayerLeft", GameLanguage.Group_PlayerLeft);
        reader.Write("Language", "Group_PlayerJoined", GameLanguage.Group_PlayerJoined);
        reader.Write("Language", "Group_InviteQuestion", GameLanguage.Group_InviteQuestion);

        // Quest related
        reader.Write("Language", "Quest_ShareQuestion", GameLanguage.Quest_ShareQuestion);
        reader.Write("Language", "Quest_TaskTitle", GameLanguage.Quest_TaskTitle);
        reader.Write("Language", "Quest_ProgressTitle", GameLanguage.Quest_ProgressTitle);
        reader.Write("Language", "Quest_ReturnTitle", GameLanguage.Quest_ReturnTitle);
        reader.Write("Language", "Quest_TimeLimitTitle", GameLanguage.Quest_TimeLimitTitle);
        reader.Write("Language", "Quest_SelectRewardRequired", GameLanguage.Quest_SelectRewardRequired);
        reader.Write("Language", "Quest_CancelConfirm", GameLanguage.Quest_CancelConfirm);

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
        reader.Write("Language", "Item_WarriorCannotUse", GameLanguage.Item_WarriorCannotUse);
        reader.Write("Language", "Item_WizardCannotUse", GameLanguage.Item_WizardCannotUse);
        reader.Write("Language", "Item_TaoistCannotUse", GameLanguage.Item_TaoistCannotUse);
        reader.Write("Language", "Item_AssassinCannotUse", GameLanguage.Item_AssassinCannotUse);
        reader.Write("Language", "Item_ArcherCannotUse", GameLanguage.Item_ArcherCannotUse);
        reader.Write("Language", "Req_NotEnoughAC", GameLanguage.Req_NotEnoughAC);
        reader.Write("Language", "Req_NotEnoughMAC", GameLanguage.Req_NotEnoughMAC);
        reader.Write("Language", "Req_MaxLevelExceeded", GameLanguage.Req_MaxLevelExceeded);
        reader.Write("Language", "Req_NotEnoughBaseAC", GameLanguage.Req_NotEnoughBaseAC);
        reader.Write("Language", "Req_NotEnoughBaseMAC", GameLanguage.Req_NotEnoughBaseMAC);
        reader.Write("Language", "Req_NotEnoughBaseDC", GameLanguage.Req_NotEnoughBaseDC);
        reader.Write("Language", "Req_NotEnoughBaseMC", GameLanguage.Req_NotEnoughBaseMC);
        reader.Write("Language", "Req_NotEnoughBaseSC", GameLanguage.Req_NotEnoughBaseSC);
        reader.Write("Language", "Req_NoMountEquipped", GameLanguage.Req_NoMountEquipped);
        reader.Write("Language", "Req_NoFishingRodEquipped", GameLanguage.Req_NoFishingRodEquipped);

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
    private static readonly Dictionary<string, string> MapNameById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, string> BuffNameById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, string> PoisonNameById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, string> StatNameById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, string> BuffDescriptionById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, string> PoisonDescriptionById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

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

    public static string GetBuffDescription(string id, string fallback)
    {
        if (string.IsNullOrEmpty(id))
            return fallback ?? string.Empty;

        string key = "BuffDesc_" + id;

        string value;
        if (BuffDescriptionById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
            return value;

        return fallback ?? string.Empty;
    }

    public static string GetPoisonDescription(string id, string fallback)
    {
        if (string.IsNullOrEmpty(id))
            return fallback ?? string.Empty;

        string key = "PoisonDesc_" + id;

        string value;
        if (PoisonDescriptionById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
            return value;

        return fallback ?? string.Empty;
    }

    public static string GetBuffName(string englishName)
    {
        if (string.IsNullOrEmpty(englishName))
            return englishName ?? string.Empty;

        string id = englishName.Replace(" ", string.Empty).Replace("(", string.Empty).Replace(")", string.Empty).Replace("-", string.Empty);
        string key = "Buff_" + id;

        string value;
        if (BuffNameById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
            return value;

        return englishName;
    }

    public static string GetPoisonName(string englishName)
    {
        if (string.IsNullOrEmpty(englishName))
            return englishName ?? string.Empty;

        string id = englishName.Replace(" ", string.Empty).Replace("(", string.Empty).Replace(")", string.Empty).Replace("-", string.Empty);
        string key = "Poison_" + id;

        string value;
        if (PoisonNameById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
            return value;

        return englishName;
    }

    public static string GetStatName(string englishName)
    {
        if (string.IsNullOrEmpty(englishName))
            return englishName ?? string.Empty;

        string id = englishName.Replace(" ", string.Empty).Replace("(", string.Empty).Replace(")", string.Empty).Replace("-", string.Empty);
        string key = "Stat_" + id;

        string value;
        if (StatNameById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
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

    public static string GetMapName(string englishName)
    {
        if (string.IsNullOrEmpty(englishName))
            return englishName ?? string.Empty;

        string id = englishName.Replace(" ", string.Empty).Replace("-", string.Empty);
        string key = "Map_" + id;

        string value;
        if (MapNameById.TryGetValue(key, out value) && !string.IsNullOrEmpty(value))
            return value;

        return englishName;
    }

    public static string GetClassName(MirClass classType)
    {
        switch (classType)
        {
            case MirClass.Warrior:
                return ClassName_Warrior;
            case MirClass.Wizard:
                return ClassName_Wizard;
            case MirClass.Taoist:
                return ClassName_Taoist;
            case MirClass.Assassin:
                return ClassName_Assassin;
            case MirClass.Archer:
                return ClassName_Archer;
            default:
                return classType.ToString();
        }
    }

    public static string GetMagicDescription(Spell spell, ClientMagic magic)
    {
        if (magic == null)
            return string.Empty;

        int nextLevelValue = magic.Level == 0 ? magic.Level1 : magic.Level == 1 ? magic.Level2 : magic.Level == 2 ? magic.Level3 : 0;

        switch (spell)
        {  //Warrior
            case Spell.Fencing:
                return string.Format("基本剑术\n\n被动技能\n\n提升基础剑术的命中率，命中会随着熟练度提高。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Slaying:
                return string.Format("攻杀剑术\n\n被动技能\n\n提高命中率与攻击力，效果会随着熟练度提升。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Thrusting:
                return string.Format("刺杀剑术\n\n切换技能\n\n延长武器攻击距离，伤害会随着熟练度提高。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Rage:
                return string.Format("狂怒\n\n增益技能\n消耗魔法：{2}\n\n激发内力，在一段时间内提升攻击力。攻击加成与持续时间随技能等级提升，施放后需要等待冷却。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.ProtectionField:
                return string.Format("护体结界\n\n增益技能\n消耗魔法：{2}\n\n凝聚内力覆盖全身，提升对敌防御。防御力与持续时间随技能等级提升，施放后需要等待冷却。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.HalfMoon:
                return string.Format("半月弯刀\n\n切换技能\n每次攻击消耗魔法：{2}\n\n挥动武器产生半月形剑气，对角色周围扇形范围内的敌人造成伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.FlamingSword:
                return string.Format("烈火剑法\n\n主动技能\n消耗魔法：{2}\n\n将火焰之力注入下一次攻击，对目标造成强力打击。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.ShoulderDash:
                return string.Format("野蛮冲撞\n\n主动技能\n消耗魔法：{2}\n\n向前猛冲撞击目标，将其击退；若目标撞上障碍物会受到额外伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.CrossHalfMoon:
                return string.Format("交叉半月\n\n切换技能\n每次攻击消耗魔法：{2}\n\n挥出两道强力半月剑气，对身旁所有敌人造成伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.TwinDrakeBlade:
                return string.Format("双龙斩\n\n主动技能\n消耗魔法：{2}\n\n施展连环强力斩击，有几率短暂眩晕目标；被眩晕的怪物会额外承受 50% 伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Entrapment:
                return string.Format("缠绕\n\n主动技能\n消耗魔法：{2}\n\n束缚并拉扯范围内的怪物靠近自身，使其行动受限。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.LionRoar:
                return string.Format("狮吼功\n\n主动技能\n消耗魔法：{2}\n\n发出强力怒吼，使周围敌人短时间麻痹；麻痹时间随技能等级增加。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.CounterAttack:
                return string.Format("反击\n\n增益技能\n消耗魔法：{2}\n\n在短时间内提升物理与魔法防御，并有几率格挡攻击并进行反击。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.ImmortalSkin:
                return string.Format("金刚不坏\n\n增益技能\n消耗魔法：{2}\n\n大幅提升防御力，减少所受伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Fury:
                return string.Format("狂暴\n\n增益技能\n消耗魔法：{2}\n\n在一段时间内提升命中率，并略微提高攻击能力。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.SlashingBurst:
                return string.Format("破空斩\n\n主动技能\n消耗魔法：{2}\n\n瞬间向前突进一格，对路径上的敌人造成伤害，可越过障碍或怪物。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.BladeAvalanche:
                return string.Format("旋风斩\n\n主动技能\n消耗魔法：{2}\n\n向前方三个方向挥舞利刃，形成致命的金属风暴攻击敌人。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);

            //Wizard
            case Spell.FireBall:
                return string.Format("火球术\n\n瞬发技能\n消耗魔法：{2}\n\n凝聚火焰之力形成火球，投向目标造成火焰伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.ThunderBolt:
                return string.Format("雷霆术\n\n瞬发技能\n消耗魔法：{2}\n\n召唤雷电击中单个目标，造成高额雷电伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.GreatFireBall:
                return string.Format("大火球\n\n瞬发技能\n消耗魔法：{2}\n\n强化版火球术，对目标造成更高的火焰伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Repulsion:
                return string.Format("抗拒火环\n\n瞬发技能\n消耗魔法：{2}\n\n以火焰之力震开周围的敌人，将其推离自身。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.HellFire:
                return string.Format("地狱火\n\n瞬发技能\n消耗魔法：{2}\n\n释放地狱火焰冲向前方，对路径上的敌人造成持续灼烧伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Lightning:
                return string.Format("疾光电影\n\n瞬发技能\n消耗魔法：{2}\n\n向前方释放一道高速雷光，对直线上的敌人造成电击伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.ElectricShock:
                return string.Format("雷电术\n\n瞬发技能\n消耗魔法：{2}\n\n以强力雷电冲击目标，使其短时间无法行动，或混乱为你而战。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Teleport:
                return string.Format("瞬息移动\n\n瞬发技能\n消耗魔法：{2}\n\n瞬间将自身传送至附近随机位置，用于躲避危机或快速位移。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.FireWall:
                return string.Format("火墙\n\n瞬发技能\n消耗魔法：{2}\n\n在指定位置召唤一堵火焰之墙，经过的敌人将持续受到火焰伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.FireBang:
                return string.Format("爆裂火焰\n\n瞬发技能\n消耗魔法：{2}\n\n在指定地点引发火焰爆炸，灼烧范围内的所有敌人。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.ThunderStorm:
                return string.Format("地狱雷光\n\n瞬发技能\n消耗魔法：{2}\n\n在自身周围降下雷暴，对范围内的亡灵或敌人造成大量雷电伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.MagicShield:
                return string.Format("魔法盾\n\n瞬发技能\n消耗魔法：{2}\n\n在自身周围形成魔法护盾，吸收一定量的伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.TurnUndead:
                return string.Format("圣言术\n\n瞬发技能\n消耗魔法：{2}\n\n对亡灵生物施展神圣之力，有几率立即将其消灭。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.IceStorm:
                return string.Format("冰咆哮\n\n瞬发技能\n消耗魔法：{2}\n\n在指定区域召唤冰风暴，对范围内的敌人造成大范围冰属性伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.FlameDisruptor:
                return string.Format("焰火分身\n\n瞬发技能\n消耗魔法：{2}\n\n从地底爆发火焰冲击波，对直线上敌人造成强烈火焰伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.FrostCrunch:
                return string.Format("冰霜嚎叫\n\n瞬发技能\n消耗魔法：{2}\n\n冻结周围空气，减缓敌人移动与攻击速度，并造成冰属性伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Mirroring:
                return string.Format("镜像术\n\n瞬发技能\n消耗魔法：{2}\n\n创造自身的镜像分身，与本体一同攻击敌人分散仇恨。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.FlameField:
                return string.Format("火焰领域\n\n瞬发技能\n消耗魔法：{2}\n\n在自身周围释放强力火焰，对附近敌人造成范围伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Vampirism:
                return string.Format("吸血术\n\n瞬发技能\n消耗魔法：{2}\n\n以魔力抽取目标生命，将部分伤害转化为自身生命值。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Blizzard:
                return string.Format("暴风雪\n\n引导技能\n消耗魔法：{2}\n\n持续召唤冰雪风暴覆盖大范围区域，对其中的敌人造成多次冰属性伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.MeteorStrike:
                return string.Format("流星坠落\n\n引导技能\n消耗魔法：{2}\n\n从天空召唤炽热流星坠落，在 5×5 范围内对敌人造成毁灭性打击。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.IceThrust:
                return string.Format("冰刺术\n\n瞬发技能\n消耗魔法：{2}\n\n在指定方向凝结冰刺突袭目标，对其造成冰属性伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.MagicBooster:
                return string.Format("魔力增幅\n\n持续效果\n消耗魔法：{2}\n\n在一定时间内提升法术伤害，但同时增加每次施法的魔法消耗。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.FastMove:
                return string.Format("疾行\n\n引导技能\n消耗魔法：{2}\n\n大幅提升移动速度，使角色在短时间内迅速移动于战场之中。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.StormEscape:
                return string.Format("风暴逃脱\n\n引导技能\n消耗魔法：{2}\n\n释放风暴之力麻痹周围敌人，并瞬间传送到指定位置。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Blink:
                return string.Format("闪烁\n\n瞬发技能\n消耗魔法：{2}\n\n瞬间传送到附近随机位置，用于灵活走位与躲避攻击。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);

            //Taoist
            case Spell.SpiritSword:
                return string.Format("精神力战法\n\n被动技能\n\n提高近身攻击的命中率。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Healing:
                return string.Format("治愈术\n\n瞬发技能\n消耗魔法：{2}\n\n对单个友方目标进行治疗，在一段时间内缓慢恢复生命值。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Poisoning:
                return string.Format("施毒术\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：毒粉\n\n向怪物投掷毒素，绿色毒素持续削减生命，红色毒素降低防御力。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.SoulFireBall:
                return string.Format("灵魂火符\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n将法力注入符纸并投向敌人，符咒爆炸造成火焰伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.SoulShield:
                return string.Format("幽灵盾\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n为自己和队友施加魔法防御护盾，提高对法术伤害的抵抗力。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.BlessedArmour:
                return string.Format("神圣战甲术\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n为自己和队友附上神圣庇护，提升物理防御力。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.TrapHexagon:
                return string.Format("困魔咒\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n以六芒封印困住目标，使其无法移动；受到外界伤害时封印将被打破。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.SummonSkeleton:
                return string.Format("召唤骷髅\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n召唤强力的骷髅战士，为你战斗并对周围敌人造成伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Hiding:
                return string.Format("隐身术\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n在短时间内隐藏自身身形，使怪物无法发现你；移动或攻击会解除隐身。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.MassHiding:
                return string.Format("集体隐身术\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n使自己与队伍成员在短时间内同时隐身；移动或攻击会解除隐身效果。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Revelation:
                return string.Format("心灵启示\n\n瞬发技能\n消耗魔法：{2}\n\n读取目标的心灵气息，可查看其生命值等关键信息。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.MassHealing:
                return string.Format("群体治疗术\n\n瞬发技能\n消耗魔法：{2}\n\n释放治疗之力，恢复指定范围内所有友方目标的生命值。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.SummonShinsu:
                return string.Format("召唤神兽\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n召唤忠诚的神兽，与自己并肩作战，持续追击敌人。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.UltimateEnhancer:
                return string.Format("极限强化\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n吸收周围能量，大幅提升自身各项属性，在持续时间内显著增强战斗能力。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.EnergyRepulsor:
                return string.Format("能量反震\n\n瞬发技能\n消耗魔法：{2}\n\n集中能量释放冲击波，将周围怪物击退并造成伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Purification:
                return string.Format("净化术\n\n瞬发技能\n消耗魔法：{2}\n\n驱散目标身上的中毒、麻痹等负面状态，恢复其正常行动能力。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.SummonHolyDeva:
                return string.Format("召唤圣灵\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n召唤强大的圣灵，以雷电之力打击敌人，协助你战斗。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Curse:
                return string.Format("诅咒术\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符 + 毒药\n\n对目标施放诅咒，降低其攻击速度以及物理、魔法与道术攻击力。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Hallucination:
                return string.Format("幻觉术\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n使怪物陷入幻觉，误以为敌人无处不在，可能攻击周围的一切目标。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Reincarnation:
                return string.Format("复活术\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n对死亡的玩家施放复活之力，使其获得重生的机会。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.PoisonCloud:
                return string.Format("毒雾术\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：绿毒\n\n投掷附带毒素的护身符，在地面制造大范围毒雾，对其中的敌人持续造成伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.EnergyShield:
                return string.Format("能量护盾\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符\n\n可对自己或友方目标施放，为其附加能量护盾，反弹部分所受伤害给攻击者。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Plague:
                return string.Format("瘟疫术\n\n瞬发技能\n消耗魔法：{2}\n\n需要物品：护身符 + 毒药\n\n使目标持续流失魔法值，并附加眩晕、诅咒、中毒、减速等随机负面状态。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.HealingCircle:
                return string.Format("治愈光环\n\n瞬发技能\n消耗魔法：{2}\n\n在自身周围展开治疗结界，为范围内友方回复生命，同时对敌人造成法术伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);

            //Assassin
            case Spell.FatalSword:
                return string.Format("致命剑法\n\n被动技能\n\n提高对怪物的攻击力，并略微提升命中率。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.DoubleSlash:
                return string.Format("双重斩\n\n切换技能\n每次攻击消耗魔法：{2}\n\n以极快的速度连续斩击两次，对目标造成连击伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Haste:
                return string.Format("疾速\n\n增益技能\n消耗魔法：{2}\n\n在一段时间内提升攻击速度，使出手更加迅捷。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.FlashDash:
                return string.Format("闪影冲刺\n\n主动技能\n消耗魔法：{2}\n\n快速突进并斩击目标，有几率使其短时间麻痹。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.HeavenlySword:
                return string.Format("天剑术\n\n主动技能\n消耗魔法：{2}\n\n释放剑气攻击自身两格范围内的敌人，对周围目标造成群体伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.FireBurst:
                return string.Format("火焰爆裂\n\n主动技能\n消耗魔法：{2}\n\n爆发周身气劲，将周围怪物击退并造成伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Trap:
                return string.Format("陷阱术\n\n瞬发技能\n冷却时间：60 秒\n消耗魔法：{2}\n\n在脚下布置陷阱，使踏入的敌人短时间无法行动。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.MoonLight:
                return string.Format("月光斩\n\n增益技能\n消耗魔法：{2}\n\n隐匿身形接近敌人，发动攻击时造成比平时更高的伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.MPEater:
                return string.Format("法力汲取\n\n被动技能\n\n从被攻击的怪物身上吸取法力，用于回复自身魔法值。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.SwiftFeet:
                return string.Format("疾风步\n\n增益技能\n消耗魔法：{2}\n\n在一段时间内提升移动速度，更加灵活地穿梭战场。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.LightBody:
                return string.Format("轻身术\n\n增益技能\n消耗魔法：{2}\n\n减轻自身重量，提高移动与闪避能力，更易躲避攻击。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.PoisonSword:
                return string.Format("毒剑术\n\n主动技能\n消耗魔法：{2}\n\n将剧毒附着在武器上，攻击时令敌人中毒并持续失去生命。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.DarkBody:
                return string.Format("暗影之躯\n\n主动技能\n消耗魔法：{2}\n\n制造自身幻象迷惑敌人，本体隐入黑暗伺机出手。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.CrescentSlash:
                return string.Format("新月斩\n\n主动技能\n消耗魔法：{2}\n\n释放新月形剑气，对自身周围的敌人造成范围伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Hemorrhage:
                return string.Format("流血打击\n\n被动技能\n\n攻击时有几率造成致命一击，并使目标进入流血状态持续失血。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.MoonMist:
                return string.Format("月影迷雾\n\n增益技能\n消耗魔法：{2}\n\n隐藏于月影迷雾之中，更容易接近敌人，并让第一击造成更强伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);

            //Archer
            case Spell.Focus:
                return string.Format("专注\n\n被动技能\n\n提高使用物理攻击时的命中率，使远程攻击更加稳定。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.StraightShot:
                return string.Format("直射\n\n主动技能\n消耗魔法：{2}\n\n以法力强化箭矢，对单个目标造成额外伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.DoubleShot:
                return string.Format("二连射\n\n主动技能\n消耗魔法：{2}\n\n快速连射两支箭矢，对目标造成连续打击。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.ExplosiveTrap:
                return string.Format("爆裂陷阱\n\n陷阱技能\n消耗魔法：{2}\n\n在地面布置一排爆裂陷阱，敌人触发时产生爆炸伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.DelayedExplosion:
                return string.Format("延时爆裂\n\n主动技能\n消耗魔法：{2}\n\n射出一支会在短暂延迟后爆炸的箭矢，可利用元素之力造成额外伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Meditation:
                return string.Format("冥想\n\n被动技能\n\n攻击怪物时有几率获得元素能量，最多可累积 4 个元素。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.BackStep:
                return string.Format("后跳\n\n主动技能\n消耗魔法：{2}\n\n迅速向后跃退，与敌人拉开距离以躲避攻击。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.ElementalShot:
                return string.Format("元素射击\n\n主动技能\n消耗魔法：{2}\n\n发射高伤害的元素箭矢，根据持有元素数量提高伤害，若无元素则生成 2 个元素。等级较高时可将目标击退。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Concentration:
                return string.Format("集中射击\n\n增益技能\n消耗魔法：{2}\n\n在技能持续时间内，提高攻击时获得元素能量的概率。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.Stonetrap:
                return string.Format("石化陷阱\n\n陷阱技能\n消耗魔法：{2}\n\n在地面放置石化陷阱，使触发的敌人短时间无法行动。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.ElementalBarrier:
                return string.Format("元素屏障\n\n增益技能\n消耗魔法：{2}\n\n以元素之力形成防护屏障，持有元素越多，减伤效果越强。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.SummonVampire:
                return string.Format("召唤吸血鬼\n\n召唤技能\n消耗魔法：{2}\n\n召唤吸血蜘蛛协助战斗，其攻击会吸取敌人生命来治疗主人。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.VampireShot:
                return string.Format("吸血箭\n\n主动技能\n消耗魔法：{2}\n\n射出带有吸血效果的箭矢，将造成伤害的一部分转化为自身生命。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.SummonToad:
                return string.Format("召唤毒蛤\n\n召唤技能\n消耗魔法：{2}\n\n召唤一只毒蛤协助战斗，它无法移动，当主人离开其视野范围时会发生爆炸。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.PoisonShot:
                return string.Format("剧毒箭\n\n主动技能\n消耗魔法：{2}\n\n射出附带剧毒的箭矢，使目标中毒并持续流失生命。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.CrippleShot:
                return string.Format("致残箭\n\n主动技能\n消耗魔法：{2}\n\n射出致残箭减缓敌人移动。若拥有剧毒箭或吸血箭的增益效果，将产生范围中毒或额外吸血效果。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.SummonSnakes:
                return string.Format("召唤毒蛇\n\n召唤技能\n消耗魔法：{2}\n\n召唤毒蛇图腾，不断召唤毒蛇群挑衅附近怪物，并有几率令其麻痹。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.NapalmShot:
                return string.Format("燃烧弹\n\n主动技能\n消耗魔法：{2}\n\n向目标射出燃烧爆裂箭，在 5×5 范围内造成火焰爆炸伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
            case Spell.OneWithNature:
                return string.Format("天人合一\n\n增益技能\n消耗魔法：{2}\n\n召唤元素之环环绕自身，对 5×5 范围内的所有敌人造成持续伤害。\n\n当前技能等级：{0}\n下一级所需熟练度：{1}", magic.Level, nextLevelValue, magic.BaseCost);
        }

        return string.Empty;
    }

    private static void LoadDatabaseTranslations(string languageIniPath)
    {
        ItemNameById.Clear();
        MonsterNameById.Clear();
        NpcNameById.Clear();
        MagicNameById.Clear();
        QuestNameById.Clear();
        MapNameById.Clear();
        BuffNameById.Clear();
        PoisonNameById.Clear();
        StatNameById.Clear();
        BuffDescriptionById.Clear();
        PoisonDescriptionById.Clear();

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
            else if (key.StartsWith("Map_", StringComparison.OrdinalIgnoreCase))
            {
                MapNameById[key] = value;
            }
            else if (key.StartsWith("Buff_", StringComparison.OrdinalIgnoreCase))
            {
                BuffNameById[key] = value;
            }
            else if (key.StartsWith("Poison_", StringComparison.OrdinalIgnoreCase))
            {
                PoisonNameById[key] = value;
            }
            else if (key.StartsWith("Stat_", StringComparison.OrdinalIgnoreCase))
            {
                StatNameById[key] = value;
            }
            else if (key.StartsWith("BuffDesc_", StringComparison.OrdinalIgnoreCase))
            {
                BuffDescriptionById[key] = value;
            }
            else if (key.StartsWith("PoisonDesc_", StringComparison.OrdinalIgnoreCase))
            {
                PoisonDescriptionById[key] = value;
            }
        }
    }
}

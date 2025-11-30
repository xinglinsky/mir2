use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;

fn parse_bool(val: &str, default: bool) -> bool {
    match val {
        "True" | "true" | "1" => true,
        "False" | "false" | "0" => false,
        _ => default,
    }
}

fn parse_u16(val: &str, default: u16) -> u16 {
    val.parse::<u16>().unwrap_or(default)
}

fn parse_u32(val: &str, default: u32) -> u32 {
    val.parse::<u32>().unwrap_or(default)
}

fn parse_i32(val: &str, default: i32) -> i32 {
    val.parse::<i32>().unwrap_or(default)
}

fn parse_i64(val: &str, default: i64) -> i64 {
    val.parse::<i64>().unwrap_or(default)
}

fn parse_f32(val: &str, default: f32) -> f32 {
    val.parse::<f32>().unwrap_or(default)
}

fn parse_u8(val: &str, default: u8) -> u8 {
    val.parse::<u8>().unwrap_or(default)
}

#[derive(Clone, Debug)]
pub struct GeneralConfig {
    pub version_path: String,
    pub check_version: bool,
    pub relog_delay: u16,
    pub gm_password: String,
    pub multithreaded: bool,
    pub thread_limit: i32,
    pub test_server: bool,
    pub enforce_db_checks: bool,
    pub monster_process_when_alone: bool,
}

#[derive(Clone, Debug)]
pub struct NetworkConfig {
    pub ip_address: String,
    pub port: u16,
    pub timeout: u16,
    pub max_user: u16,
    pub max_ip: u16,
    pub max_packet: u16,
    pub start_http_service: bool,
    pub http_ip_address: String,
    pub http_trusted_ip_address: String,
}

#[derive(Clone, Debug)]
pub struct PermissionConfig {
    pub allow_new_account: bool,
    pub allow_change_password: bool,
    pub allow_login: bool,
    pub allow_new_character: bool,
    pub allow_delete_character: bool,
    pub allow_start_game: bool,
    pub allow_create_assassin: bool,
    pub allow_create_archer: bool,
    pub max_resolution: i32,
}

#[derive(Clone, Debug)]
pub struct OptionalConfig {
    pub gather_orbs_per_level: bool,
    pub exp_mob_level_difference: bool,
    pub line_message_timer: i32,
}

#[derive(Clone, Debug)]
pub struct DatabaseConfig {
    pub save_delay: i32,
    pub credx_gold: i16,
}

#[derive(Clone, Debug)]
pub struct GameConfig {
    pub exp_rate: f32,
    pub item_timeout: i32,
    pub player_died_item_timeout: i32,
    pub pet_save: bool,
    pub pk_delay: i32,
    pub newbie_guild: String,
    pub newbie_guild_max_size: i32,
    /// Base gold required to rent a Guild Territory (BuyGTGold in C# Settings).
    pub buy_gt_gold: i32,
    /// Gold required to extend an existing Guild Territory rental (ExtendGT in C# Settings).
    pub extend_gt_gold: i32,
    /// Number of days added when extending a Guild Territory rental (GTDays in C# Settings).
    pub gt_days: i32,
}

#[derive(Clone, Debug)]
pub struct RestedConfig {
    pub period: i32,
    pub buff_length: i32,
    pub exp_bonus: i32,
    pub max_bonus: i32,
}

#[derive(Clone, Debug)]
pub struct ItemsConfig {
    pub heal_ring: String,
    pub fire_ring: String,
    pub blink_skill: String,
    pub magic_resist_weight: u8,
    pub poison_resist_weight: u8,
    pub critical_rate_weight: u8,
    pub critical_damage_weight: u8,
    pub freezing_attack_weight: u8,
    pub poison_attack_weight: u8,
    pub health_regen_weight: u8,
    pub mana_regen_weight: u8,
    pub max_luck: u8,
    pub seal_delay: u16,
    pub pvp_can_resist_magic: bool,
    pub pvp_can_resist_poison: bool,
    pub pvp_can_freeze: bool,
}

#[derive(Clone, Debug)]
pub struct PKTownConfig {
    pub map_name: String,
    pub position_x: i32,
    pub position_y: i32,
}

#[derive(Clone, Debug)]
pub struct DropGoldConfig {
    pub drop_gold: bool,
    pub max_drop_gold: u32,
}

#[derive(Clone, Debug)]
pub struct BonusConfig {
    pub range_accuracy_bonus: u8,
}

#[derive(Clone, Debug)]
pub struct IntelligentCreaturesConfig {
    pub creature_black_stone_name: String,
}

#[derive(Clone, Debug)]
pub struct ObserveConfig {
    pub allow_observe: bool,
}

#[derive(Clone, Debug)]
pub struct ArchiveConfig {
    pub inactive_character_months: i32,
    pub deleted_character_months: i32,
}

#[derive(Clone, Debug)]
pub struct SetupConfig {
    pub optional: OptionalConfig,
    pub game: GameConfig,
    pub rested: RestedConfig,
    pub items: ItemsConfig,
    pub pktown: PKTownConfig,
    pub drop_gold: DropGoldConfig,
    pub bonus: BonusConfig,
    pub intelligent_creatures: IntelligentCreaturesConfig,
    pub observe: ObserveConfig,
    pub archive: ArchiveConfig,
}

impl Default for SetupConfig {
    fn default() -> Self {
        SetupConfig {
            optional: OptionalConfig {
                gather_orbs_per_level: true,
                exp_mob_level_difference: true,
                line_message_timer: 10,
            },
            game: GameConfig {
                exp_rate: 1.0,
                item_timeout: 30,
                player_died_item_timeout: 120,
                pet_save: false,
                pk_delay: 12,
                newbie_guild: "NewbieGuild".to_string(),
                newbie_guild_max_size: 1000,
                buy_gt_gold: 10_000_000,
                extend_gt_gold: 1_000_000,
                gt_days: 30,
            },
            rested: RestedConfig {
                period: 60,
                buff_length: 10,
                exp_bonus: 5,
                max_bonus: 24,
            },
            items: ItemsConfig {
                heal_ring: "Healing".to_string(),
                fire_ring: "FireBall".to_string(),
                blink_skill: "Blink".to_string(),
                magic_resist_weight: 10,
                poison_resist_weight: 10,
                critical_rate_weight: 5,
                critical_damage_weight: 50,
                freezing_attack_weight: 10,
                poison_attack_weight: 10,
                health_regen_weight: 10,
                mana_regen_weight: 10,
                max_luck: 10,
                seal_delay: 60,
                pvp_can_resist_magic: false,
                pvp_can_resist_poison: false,
                pvp_can_freeze: false,
            },
            pktown: PKTownConfig {
                map_name: "3".to_string(),
                position_x: 848,
                position_y: 677,
            },
            drop_gold: DropGoldConfig {
                drop_gold: true,
                max_drop_gold: 2000,
            },
            bonus: BonusConfig {
                range_accuracy_bonus: 0,
            },
            intelligent_creatures: IntelligentCreaturesConfig {
                creature_black_stone_name: "BlackCreatureStone".to_string(),
            },
            observe: ObserveConfig {
                allow_observe: false,
            },
            archive: ArchiveConfig {
                inactive_character_months: 1,
                deleted_character_months: 1,
            },
        }
    }
}

pub static SETUP_CONFIG: Lazy<SetupConfig> = Lazy::new(|| {
    load_setup_config(Path::new("./Configs/Setup.ini"))
});

pub fn setup_config() -> &'static SetupConfig {
    &SETUP_CONFIG
}

fn load_setup_config(path: &Path) -> SetupConfig {
    let mut cfg = SetupConfig::default();

    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return cfg,
    };

    let mut current_section: Option<String> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let name = &line[1..line.len() - 1];
            current_section = Some(name.to_string());
            continue;
        }

        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }

        match current_section.as_deref() {
            Some("Optional") => match key {
                "GatherOrbsPerLevel" => {
                    cfg.optional.gather_orbs_per_level =
                        parse_bool(value, cfg.optional.gather_orbs_per_level)
                }
                "ExpMobLevelDifference" => {
                    cfg.optional.exp_mob_level_difference =
                        parse_bool(value, cfg.optional.exp_mob_level_difference)
                }
                "LineMessageTimer" => {
                    cfg.optional.line_message_timer =
                        parse_i32(value, cfg.optional.line_message_timer)
                }
                _ => {}
            },
            Some("Game") => match key {
                "ExpRate" => cfg.game.exp_rate = parse_f32(value, cfg.game.exp_rate),
                "ItemTimeOut" => cfg.game.item_timeout = parse_i32(value, cfg.game.item_timeout),
                "PlayerDiedItemTimeOut" => {
                    cfg.game.player_died_item_timeout =
                        parse_i32(value, cfg.game.player_died_item_timeout)
                }
                "PetSave" => cfg.game.pet_save = parse_bool(value, cfg.game.pet_save),
                "PKDelay" => cfg.game.pk_delay = parse_i32(value, cfg.game.pk_delay),
                "NewbieGuild" => cfg.game.newbie_guild = value.to_string(),
                "NewbieGuildMaxSize" => {
                    cfg.game.newbie_guild_max_size =
                        parse_i32(value, cfg.game.newbie_guild_max_size)
                }
                "BuyGTGold" => {
                    cfg.game.buy_gt_gold = parse_i32(value, cfg.game.buy_gt_gold)
                }
                "ExtendGT" => {
                    cfg.game.extend_gt_gold = parse_i32(value, cfg.game.extend_gt_gold)
                }
                "GTDays" => {
                    cfg.game.gt_days = parse_i32(value, cfg.game.gt_days)
                }
                _ => {}
            },
            Some("Rested") => match key {
                "Period" => cfg.rested.period = parse_i32(value, cfg.rested.period),
                "BuffLength" => cfg.rested.buff_length = parse_i32(value, cfg.rested.buff_length),
                "ExpBonus" => cfg.rested.exp_bonus = parse_i32(value, cfg.rested.exp_bonus),
                "MaxBonus" => cfg.rested.max_bonus = parse_i32(value, cfg.rested.max_bonus),
                _ => {}
            },
            Some("Items") => match key {
                "HealRing" => cfg.items.heal_ring = value.to_string(),
                "FireRing" => cfg.items.fire_ring = value.to_string(),
                "BlinkSkill" => cfg.items.blink_skill = value.to_string(),
                "MagicResistWeight" => cfg.items.magic_resist_weight = parse_u8(value, cfg.items.magic_resist_weight),
                "PoisonResistWeight" => cfg.items.poison_resist_weight = parse_u8(value, cfg.items.poison_resist_weight),
                "CriticalRateWeight" => cfg.items.critical_rate_weight = parse_u8(value, cfg.items.critical_rate_weight),
                "CriticalDamageWeight" => cfg.items.critical_damage_weight = parse_u8(value, cfg.items.critical_damage_weight),
                "FreezingAttackWeight" => cfg.items.freezing_attack_weight = parse_u8(value, cfg.items.freezing_attack_weight),
                "PoisonAttackWeight" => cfg.items.poison_attack_weight = parse_u8(value, cfg.items.poison_attack_weight),
                "HealthRegenWeight" => cfg.items.health_regen_weight = parse_u8(value, cfg.items.health_regen_weight),
                "ManaRegenWeight" => cfg.items.mana_regen_weight = parse_u8(value, cfg.items.mana_regen_weight),
                "MaxLuck" => cfg.items.max_luck = parse_u8(value, cfg.items.max_luck),
                "SealDelay" => cfg.items.seal_delay = value.parse::<u16>().unwrap_or(cfg.items.seal_delay),
                "PvpCanResistMagic" => cfg.items.pvp_can_resist_magic = parse_bool(value, cfg.items.pvp_can_resist_magic),
                "PvpCanResistPoison" => cfg.items.pvp_can_resist_poison = parse_bool(value, cfg.items.pvp_can_resist_poison),
                "PvpCanFreeze" => cfg.items.pvp_can_freeze = parse_bool(value, cfg.items.pvp_can_freeze),
                _ => {}
            },
            Some("PKTown") => match key {
                "PKTownMapName" => cfg.pktown.map_name = value.to_string(),
                "PKTownPositionX" => cfg.pktown.position_x = parse_i32(value, cfg.pktown.position_x),
                "PKTownPositionY" => cfg.pktown.position_y = parse_i32(value, cfg.pktown.position_y),
                _ => {}
            },
            Some("DropGold") => match key {
                "DropGold" => cfg.drop_gold.drop_gold = parse_bool(value, cfg.drop_gold.drop_gold),
                "MaxDropGold" => cfg.drop_gold.max_drop_gold = parse_u32(value, cfg.drop_gold.max_drop_gold),
                _ => {}
            },
            Some("Bonus") => match key {
                "RangeAccuracyBonus" => cfg.bonus.range_accuracy_bonus = parse_u8(value, cfg.bonus.range_accuracy_bonus),
                _ => {}
            },
            Some("IntelligentCreatures") => match key {
                "CreatureBlackStoneName" => cfg.intelligent_creatures.creature_black_stone_name = value.to_string(),
                _ => {}
            },
            Some("Observe") => match key {
                "AllowObserve" => cfg.observe.allow_observe = parse_bool(value, cfg.observe.allow_observe),
                _ => {}
            },
            Some("Archive") => match key {
                "InactiveCharacterMonths" => cfg.archive.inactive_character_months = parse_i32(value, cfg.archive.inactive_character_months),
                "DeletedCharacterMonths" => cfg.archive.deleted_character_months = parse_i32(value, cfg.archive.deleted_character_months),
                _ => {}
            },
            _ => {}
        }
    }

    cfg
}

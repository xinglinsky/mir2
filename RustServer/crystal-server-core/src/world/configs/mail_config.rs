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

fn parse_u32(val: &str, default: u32) -> u32 {
    val.parse::<u32>().unwrap_or(default)
}

#[derive(Clone, Debug)]
pub struct MailAutoSendConfig {
    pub gold: bool,
    pub items: bool,
}

#[derive(Clone, Debug)]
pub struct MailRatesConfig {
    pub free_with_stamp: bool,
    pub cost_per_1k: u32,
    pub insurance_per_item: u32,
}

#[derive(Clone, Debug)]
pub struct MailConfig {
    pub auto_send: MailAutoSendConfig,
    pub rates: MailRatesConfig,
    pub mail_capacity: u32,
}

impl Default for MailConfig {
    fn default() -> Self {
        MailConfig {
            auto_send: MailAutoSendConfig {
                gold: false,
                items: false,
            },
            rates: MailRatesConfig {
                free_with_stamp: true,
                cost_per_1k: 100,
                insurance_per_item: 5,
            },
            mail_capacity: 100,
        }
    }
}

pub static MAIL_CONFIG: Lazy<MailConfig> =
    Lazy::new(|| load_mail_config(Path::new("./Configs/MailSystem.ini")));

pub fn mail_config() -> &'static MailConfig {
    &MAIL_CONFIG
}

fn load_mail_config(path: &Path) -> MailConfig {
    let mut cfg = MailConfig::default();

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
            current_section = Some(line[1..line.len() - 1].to_string());
            continue;
        }

        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }

        match current_section.as_deref() {
            Some("AutoSend") => match key {
                "Gold" => {
                    cfg.auto_send.gold = parse_bool(value, cfg.auto_send.gold);
                }
                "Items" => {
                    cfg.auto_send.items = parse_bool(value, cfg.auto_send.items);
                }
                _ => {}
            },
            Some("Rates") => match key {
                "FreeWithStamp" => {
                    cfg.rates.free_with_stamp = parse_bool(value, cfg.rates.free_with_stamp);
                }
                "CostPer1k" => {
                    cfg.rates.cost_per_1k = parse_u32(value, cfg.rates.cost_per_1k);
                }
                "InsurancePerItem" => {
                    cfg.rates.insurance_per_item =
                        parse_u32(value, cfg.rates.insurance_per_item);
                }
                _ => {}
            },
            Some("General") => match key {
                "MailCapacity" => {
                    cfg.mail_capacity = parse_u32(value, cfg.mail_capacity);
                }
                _ => {}
            },
            _ => {}
        }
    }

    cfg
}

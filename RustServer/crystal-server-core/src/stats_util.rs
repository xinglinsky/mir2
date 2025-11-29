use crate::stats::{Stat, Stats};
use crystal_shared_proto::item_types::{ItemInfoData, StatsMap, UserItemData};

/// Convert a shared-proto StatsMap (u8 stat id, i32 value) into a core Stats
/// using the same mapping as Stat::from_u8. This mirrors how C# reads stat
/// bytes from ItemInfo/UserItem into its Stats collection.
pub fn stats_from_map(map: &StatsMap) -> Stats {
    let mut stats = Stats::default();
    for (id, value) in &map.entries {
        let stat = Stat::from_u8(*id);
        if *value != 0 {
            stats.set(stat, *value);
        }
    }
    stats
}

/// Apply awakening (Awake) bonuses for a single equipped item, mirroring the
/// effect of C# Awake.GetDC/MC/SC/AC/MAC/HPMP as used in RefreshEquipmentStats.
fn apply_awake_stats(stats: &mut Stats, info: &ItemInfoData, user: &UserItemData) {
    // In the original C# implementation, awakening is only meaningful for
    // items that can be awakened and that actually have at least one awake
    // value recorded. The Rust shared-proto represents this as an AwakeData
    // with a non-zero awake_type and a non-empty values list.
    let awake = &user.awake;
    if awake.values.is_empty() {
        return;
    }

    if !info.can_awakening {
        return;
    }

    let total: i32 = awake.values.iter().map(|v| *v as i32).sum();
    if total <= 0 {
        return;
    }

    match awake.awake_type {
        // AwakeType::DC
        1 => {
            let min_dc = stats.get(Stat::MinDC).saturating_add(total);
            let max_dc = stats.get(Stat::MaxDC).saturating_add(total);
            stats.set(Stat::MinDC, min_dc);
            stats.set(Stat::MaxDC, max_dc);
        }
        // AwakeType::MC
        2 => {
            let min_mc = stats.get(Stat::MinMC).saturating_add(total);
            let max_mc = stats.get(Stat::MaxMC).saturating_add(total);
            stats.set(Stat::MinMC, min_mc);
            stats.set(Stat::MaxMC, max_mc);
        }
        // AwakeType::SC
        3 => {
            let min_sc = stats.get(Stat::MinSC).saturating_add(total);
            let max_sc = stats.get(Stat::MaxSC).saturating_add(total);
            stats.set(Stat::MinSC, min_sc);
            stats.set(Stat::MaxSC, max_sc);
        }
        // AwakeType::AC
        4 => {
            let min_ac = stats.get(Stat::MinAC).saturating_add(total);
            let max_ac = stats.get(Stat::MaxAC).saturating_add(total);
            stats.set(Stat::MinAC, min_ac);
            stats.set(Stat::MaxAC, max_ac);
        }
        // AwakeType::MAC
        5 => {
            let min_mac = stats.get(Stat::MinMAC).saturating_add(total);
            let max_mac = stats.get(Stat::MaxMAC).saturating_add(total);
            stats.set(Stat::MinMAC, min_mac);
            stats.set(Stat::MaxMAC, max_mac);
        }
        // AwakeType::HPMP – add the total to both HP and MP.
        6 => {
            let hp = stats.get(Stat::HP).saturating_add(total);
            let mp = stats.get(Stat::MP).saturating_add(total);
            stats.set(Stat::HP, hp);
            stats.set(Stat::MP, mp);
        }
        _ => {}
    }
}

/// Aggregate the stats contributed by a single equipped item, mirroring the
/// additive parts of C# RefreshEquipmentStats:
///
/// - base ItemInfo.stats
/// - UserItem.added_stats
/// - Awake bonuses (DC/MC/SC/AC/MAC/HPMP)
///
/// Socketed items, set bonuses and special flags are still modelled
/// separately; this helper focuses on the per-item contributions only.
pub fn aggregated_item_stats(info: &ItemInfoData, user: &UserItemData) -> Stats {
    let mut stats = Stats::default();

    let base_stats = stats_from_map(&info.stats);
    stats.add(&base_stats);

    let added = stats_from_map(&user.added_stats);
    stats.add(&added);

    // Apply awakening bonuses so that server-side HP/MP and offensive/defensive
    // stats include the same per-item Awake contributions that the C# server
    // and client use when building their Stats collections.
    apply_awake_stats(&mut stats, info, user);

    stats
}

/// Aggregate stats for a list of equipped items. The caller is responsible for
/// choosing which UserItemData entries are actually equipped and for looking up
/// the corresponding ItemInfoData in the WorldProvider's item table.
///
/// This function does not perform any filtering based on RequiredClass, level
/// requirements, or durability; it simply mirrors the additive behaviour of
/// Stats.Add in C# once an item has been accepted as equipped.
pub fn aggregate_equipment_stats(equipped: &[(ItemInfoData, UserItemData)]) -> Stats {
    let mut stats = Stats::default();
    for (info, user) in equipped {
        let item_stats = aggregated_item_stats(info, user);
        stats.add(&item_stats);
    }
    stats
}

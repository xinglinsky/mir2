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

/// Aggregate the stats contributed by a single equipped item, mirroring the
/// additive parts of C# RefreshEquipmentStats:
///
/// - base ItemInfo.stats
/// - UserItem.added_stats
///
/// Awakening, sockets, set bonuses and special flags are intentionally NOT
/// handled here; they should be modelled separately when the corresponding
/// systems are implemented.
pub fn aggregated_item_stats(info: &ItemInfoData, user: &UserItemData) -> Stats {
    let mut stats = Stats::default();

    let base_stats = stats_from_map(&info.stats);
    stats.add(&base_stats);

    let added = stats_from_map(&user.added_stats);
    stats.add(&added);

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

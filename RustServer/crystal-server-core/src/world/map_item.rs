use crystal_shared_proto::item_types::UserItemData;

#[derive(Clone, Debug)]
pub struct MapItem {
    /// Unique identifier for this map item within the world. This value is
    /// used as the object_id field in scene packets such as SObjectItem and
    /// SObjectGold (cast to u32 on the wire).
    pub id: u64,
    pub map_index: i32,
    pub x: i32,
    pub y: i32,
    /// If Some, this represents a concrete item from ItemInfoData with the
    /// given index. If None, this entry represents pure gold on the ground.
    pub item_index: Option<i32>,
    pub gold: u32,
    pub count: u16,
    pub item: Option<UserItemData>,
    /// Time (in milliseconds since Unix epoch) when this item should expire
    /// and be removed from the map. This mirrors C# ItemObject.ExpireTime.
    pub expire_time_ms: i64,
}

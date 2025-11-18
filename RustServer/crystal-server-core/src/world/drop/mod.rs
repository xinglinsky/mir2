#[derive(Clone, Debug)]
pub struct DropInfo {
    pub chance: i32,
    /// If None, this drop is pure gold or a group-only marker.
    pub item_index: Option<i32>,
    pub gold: u32,
    /// Optional nested group of drops (mirrors C# GroupDropInfo usage).
    pub grouped_drop: Option<GroupDropInfo>,
    /// C# DropInfo.Type
    pub drop_type: u8,
    pub quest_required: bool,
}

#[derive(Clone, Debug)]
pub struct GroupDropInfo {
    pub random: bool,
    pub first: bool,
    pub drops: Vec<DropInfo>,
}

#[derive(Clone, Debug)]
pub struct DropRewardInfo {
    pub items: Vec<i32>,
    pub gold: u32,
}

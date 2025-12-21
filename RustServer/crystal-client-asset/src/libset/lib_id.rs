//! Lib 文件 ID 定义

/// Lib 文件标识符
///
/// 枚举所有客户端使用的 Lib 文件
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LibId {
    /// 选角界面背景
    ChrSel,
    
    /// 程序使用（UI 元素）
    Prguse,
    
    /// 标题界面
    Title,
    
    /// 物品图标
    Items,
    
    /// 状态图标
    StateItem,
    
    /// 技能图标
    Magic,
    
    /// 魔法效果
    Magic2,
    
    /// 魔法效果 2
    Magic3,
    
    /// 魔法效果 3
    Magic4,
    
    /// 魔法效果 4
    Magic5,
    
    /// 魔法效果 5
    Magic6,
    
    /// 魔法效果 6
    Magic7,
    
    /// 魔法效果 7
    Magic8,
    
    /// 魔法效果 8
    Magic9,
    
    /// 魔法效果 9
    Magic10,
    
    /// 魔法效果 10
    Magic11,
    
    /// 魔法效果 11
    Magic12,
    
    /// 魔法效果 12
    Magic13,
    
    /// 魔法效果 13
    Magic14,
    
    /// 魔法效果 14
    Magic15,
    
    /// 魔法效果 15
    Magic16,
    
    /// 魔法效果 16
    Magic17,
    
    /// 魔法效果 17
    Magic18,
    
    /// 魔法效果 18
    Magic19,
    
    /// 魔法效果 19
    Magic20,
    
    /// 魔法效果 20
    Magic21,
    
    /// 魔法效果 21
    Magic22,
    
    /// 魔法效果 22
    Magic23,
    
    /// 魔法效果 23
    Magic24,
    
    /// 魔法效果 24
    Magic25,
    
    /// 魔法效果 25
    Magic26,
    
    /// 魔法效果 26
    Magic27,
    
    /// 魔法效果 27
    Magic28,
    
    /// 魔法效果 28
    Magic29,
    
    /// 魔法效果 29
    Magic30,
    
    /// 魔法效果 30
    Magic31,
    
    /// 魔法效果 31
    Magic32,
    
    /// 魔法效果 32
    Magic33,
    
    /// 魔法效果 33
    Magic34,
    
    /// 魔法效果 34
    Magic35,
    
    /// 魔法效果 35
    Magic36,
    
    /// 魔法效果 36
    Magic37,
    
    /// 魔法效果 37
    Magic38,
    
    /// 魔法效果 38
    Magic39,
    
    /// 魔法效果 39
    Magic40,
    
    /// 魔法效果 40
    Magic41,
    
    /// 魔法效果 41
    Magic42,
    
    /// 魔法效果 42
    Magic43,
    
    /// 魔法效果 43
    Magic44,
    
    /// 魔法效果 44
    Magic45,
    
    /// 魔法效果 45
    Magic46,
    
    /// 魔法效果 46
    Magic47,
    
    /// 魔法效果 47
    Magic48,
    
    /// 魔法效果 48
    Magic49,
    
    /// 魔法效果 49
    Magic50,
    
    /// 魔法效果 50
    Magic51,
    
    /// 魔法效果 51
    Magic52,
    
    /// 魔法效果 52
    Magic53,
    
    /// 魔法效果 53
    Magic54,
    
    /// 魔法效果 54
    Magic55,
    
    /// 魔法效果 55
    Magic56,
    
    /// 魔法效果 56
    Magic57,
    
    /// 魔法效果 57
    Magic58,
    
    /// 魔法效果 58
    Magic59,
    
    /// 魔法效果 59
    Magic60,
    
    /// 魔法效果 60
    Magic61,
    
    /// 魔法效果 61
    Magic62,
    
    /// 魔法效果 62
    Magic63,
    
    /// 魔法效果 63
    Magic64,
    
    /// 魔法效果 64
    Magic65,
    
    /// 魔法效果 65
    Magic66,
    
    /// 魔法效果 66
    Magic67,
    
    /// 魔法效果 67
    Magic68,
    
    /// 魔法效果 68
    Magic69,
    
    /// 魔法效果 69
    Magic70,
    
    /// 魔法效果 70
    Magic71,
    
    /// 魔法效果 71
    Magic72,
    
    /// 魔法效果 72
    Magic73,
    
    /// 魔法效果 73
    Magic74,
    
    /// 魔法效果 74
    Magic75,
    
    /// 魔法效果 75
    Magic76,
    
    /// 魔法效果 76
    Magic77,
    
    /// 魔法效果 77
    Magic78,
    
    /// 魔法效果 78
    Magic79,
    
    /// 魔法效果 79
    Magic80,
    
    /// 魔法效果 80
    Magic81,
    
    /// 魔法效果 81
    Magic82,
    
    /// 魔法效果 82
    Magic83,
    
    /// 魔法效果 83
    Magic84,
    
    /// 魔法效果 84
    Magic85,
    
    /// 魔法效果 85
    Magic86,
    
    /// 魔法效果 86
    Magic87,
    
    /// 魔法效果 87
    Magic88,
    
    /// 魔法效果 88
    Magic89,
    
    /// 魔法效果 89
    Magic90,
    
    /// 魔法效果 90
    Magic91,
    
    /// 魔法效果 91
    Magic92,
    
    /// 魔法效果 92
    Magic93,
    
    /// 魔法效果 93
    Magic94,
    
    /// 魔法效果 94
    Magic95,
    
    /// 魔法效果 95
    Magic96,
    
    /// 魔法效果 96
    Magic97,
    
    /// 魔法效果 97
    Magic98,
    
    /// 魔法效果 98
    Magic99,
    
    /// 魔法效果 99
    Magic100,
    
    /// 魔法效果 100
    Magic101,
    
    /// 魔法效果 101
    Magic102,
    
    /// 魔法效果 102
    Magic103,
    
    /// 魔法效果 103
    Magic104,
    
    /// 魔法效果 104
    Magic105,
    
    /// 魔法效果 105
    Magic106,
    
    /// 魔法效果 106
    Magic107,
    
    /// 魔法效果 107
    Magic108,
    
    /// 魔法效果 108
    Magic109,
    
    /// 魔法效果 109
    Magic110,
    
    /// 魔法效果 110
    Magic111,
    
    /// 魔法效果 111
    Magic112,
    
    /// 魔法效果 112
    Magic113,
    
    /// 魔法效果 113
    Magic114,
    
    /// 魔法效果 114
    Magic115,
    
    /// 魔法效果 115
    Magic116,
    
    /// 魔法效果 116
    Magic117,
    
    /// 魔法效果 117
    Magic118,
    
    /// 魔法效果 118
    Magic119,
    
    /// 魔法效果 119
    Magic120,
    
    /// 魔法效果 120
    Magic121,
    
    /// 魔法效果 121
    Magic122,
    
    /// 魔法效果 122
    Magic123,
    
    /// 魔法效果 123
    Magic124,
    
    /// 魔法效果 124
    Magic125,
    
    /// 魔法效果 125
    Magic126,
    
    /// 魔法效果 126
    Magic127,
    
    /// 魔法效果 127
    Magic128,
    
    /// 魔法效果 128
    Magic129,
    
    /// 魔法效果 129
    Magic130,
    
    /// 魔法效果 130
    Magic131,
    
    /// 魔法效果 131
    Magic132,
    
    /// 魔法效果 132
    Magic133,
    
    /// 魔法效果 133
    Magic134,
    
    /// 魔法效果 134
    Magic135,
    
    /// 魔法效果 135
    Magic136,
    
    /// 魔法效果 136
    Magic137,
    
    /// 魔法效果 137
    Magic138,
    
    /// 魔法效果 138
    Magic139,
    
    /// 魔法效果 139
    Magic140,
    
    /// 魔法效果 140
    Magic141,
    
    /// 魔法效果 141
    Magic142,
    
    /// 魔法效果 142
    Magic143,
    
    /// 魔法效果 143
    Magic144,
    
    /// 魔法效果 144
    Magic145,
    
    /// 魔法效果 145
    Magic146,
    
    /// 魔法效果 146
    Magic147,
    
    /// 魔法效果 147
    Magic148,
    
    /// 魔法效果 148
    Magic149,
    
    /// 魔法效果 149
    Magic150,
    
    /// 魔法效果 150
    Magic151,
    
    /// 魔法效果 151
    Magic152,
    
    /// 魔法效果 152
    Magic153,
    
    /// 魔法效果 153
    Magic154,
    
    /// 魔法效果 154
    Magic155,
    
    /// 魔法效果 155
    Magic156,
    
    /// 魔法效果 156
    Magic157,
    
    /// 魔法效果 157
    Magic158,
    
    /// 魔法效果 158
    Magic159,
    
    /// 魔法效果 159
    Magic160,
    
    /// 魔法效果 160
    Magic161,
    
    /// 魔法效果 161
    Magic162,
    
    /// 魔法效果 162
    Magic163,
    
    /// 魔法效果 163
    Magic164,
    
    /// 魔法效果 164
    Magic165,
    
    /// 魔法效果 165
    Magic166,
    
    /// 魔法效果 166
    Magic167,
    
    /// 魔法效果 167
    Magic168,
    
    /// 魔法效果 168
    Magic169,
    
    /// 魔法效果 169
    Magic170,
    
    /// 魔法效果 170
    Magic171,
    
    /// 魔法效果 171
    Magic172,
    
    /// 魔法效果 172
    Magic173,
    
    /// 魔法效果 173
    Magic174,
    
    /// 魔法效果 174
    Magic175,
    
    /// 魔法效果 175
    Magic176,
    
    /// 魔法效果 176
    Magic177,
    
    /// 魔法效果 177
    Magic178,
    
    /// 魔法效果 178
    Magic179,
    
    /// 魔法效果 179
    Magic180,
    
    /// 魔法效果 180
    Magic181,
    
    /// 魔法效果 181
    Magic182,
    
    /// 魔法效果 182
    Magic183,
    
    /// 魔法效果 183
    Magic184,
    
    /// 魔法效果 184
    Magic185,
    
    /// 魔法效果 185
    Magic186,
    
    /// 魔法效果 186
    Magic187,
    
    /// 魔法效果 187
    Magic188,
    
    /// 魔法效果 188
    Magic189,
    
    /// 魔法效果 189
    Magic190,
    
    /// 魔法效果 190
    Magic191,
    
    /// 魔法效果 191
    Magic192,
    
    /// 魔法效果 192
    Magic193,
    
    /// 魔法效果 193
    Magic194,
    
    /// 魔法效果 194
    Magic195,
    
    /// 魔法效果 195
    Magic196,
    
    /// 魔法效果 196
    Magic197,
    
    /// 魔法效果 197
    Magic198,
    
    /// 魔法效果 198
    Magic199,
    
    /// 魔法效果 199
    Magic200,
    
    // 其他常用 Lib 文件可以后续添加
}

/// 从文件名获取 LibId（如果存在）
pub fn from_filename(filename: &str) -> Option<LibId> {
    match filename {
        "ChrSel.Lib" => Some(LibId::ChrSel),
        "Prguse.Lib" => Some(LibId::Prguse),
        "Title.Lib" => Some(LibId::Title),
        "Items.Lib" => Some(LibId::Items),
        "StateItem.Lib" => Some(LibId::StateItem),
        "Magic.Lib" => Some(LibId::Magic),
        _ => {
            // 尝试匹配 Magic*.Lib 格式
            if filename.starts_with("Magic") && filename.ends_with(".Lib") {
                let num_str = &filename[5..filename.len() - 4];
                    if let Ok(_num) = num_str.parse::<u32>() {
                        // TODO: 动态匹配 Magic2-Magic200
                        // 当前简化处理，只返回 Magic2 作为示例
                        // 实际实现中可以使用宏生成所有 Magic 变体，或使用动态查找
                        if _num == 2 {
                            Some(LibId::Magic2)
                        } else {
                            // 其他 Magic 文件暂不支持动态匹配
                            None
                        }
                    } else {
                        None
                    }
            } else {
                None
            }
        }
    }
}

impl LibId {
    /// 获取 Lib 文件名
    pub fn filename(&self) -> &'static str {
        match self {
            LibId::ChrSel => "ChrSel.Lib",
            LibId::Prguse => "Prguse.Lib",
            LibId::Title => "Title.Lib",
            LibId::Items => "Items.Lib",
            LibId::StateItem => "StateItem.Lib",
            LibId::Magic => "Magic.Lib",
            LibId::Magic2 => "Magic2.Lib",
            LibId::Magic3 => "Magic3.Lib",
            LibId::Magic4 => "Magic4.Lib",
            LibId::Magic5 => "Magic5.Lib",
            LibId::Magic6 => "Magic6.Lib",
            LibId::Magic7 => "Magic7.Lib",
            LibId::Magic8 => "Magic8.Lib",
            LibId::Magic9 => "Magic9.Lib",
            LibId::Magic10 => "Magic10.Lib",
            LibId::Magic11 => "Magic11.Lib",
            LibId::Magic12 => "Magic12.Lib",
            LibId::Magic13 => "Magic13.Lib",
            LibId::Magic14 => "Magic14.Lib",
            LibId::Magic15 => "Magic15.Lib",
            LibId::Magic16 => "Magic16.Lib",
            LibId::Magic17 => "Magic17.Lib",
            LibId::Magic18 => "Magic18.Lib",
            LibId::Magic19 => "Magic19.Lib",
            LibId::Magic20 => "Magic20.Lib",
            LibId::Magic21 => "Magic21.Lib",
            LibId::Magic22 => "Magic22.Lib",
            LibId::Magic23 => "Magic23.Lib",
            LibId::Magic24 => "Magic24.Lib",
            LibId::Magic25 => "Magic25.Lib",
            LibId::Magic26 => "Magic26.Lib",
            LibId::Magic27 => "Magic27.Lib",
            LibId::Magic28 => "Magic28.Lib",
            LibId::Magic29 => "Magic29.Lib",
            LibId::Magic30 => "Magic30.Lib",
            LibId::Magic31 => "Magic31.Lib",
            LibId::Magic32 => "Magic32.Lib",
            LibId::Magic33 => "Magic33.Lib",
            LibId::Magic34 => "Magic34.Lib",
            LibId::Magic35 => "Magic35.Lib",
            LibId::Magic36 => "Magic36.Lib",
            LibId::Magic37 => "Magic37.Lib",
            LibId::Magic38 => "Magic38.Lib",
            LibId::Magic39 => "Magic39.Lib",
            LibId::Magic40 => "Magic40.Lib",
            LibId::Magic41 => "Magic41.Lib",
            LibId::Magic42 => "Magic42.Lib",
            LibId::Magic43 => "Magic43.Lib",
            LibId::Magic44 => "Magic44.Lib",
            LibId::Magic45 => "Magic45.Lib",
            LibId::Magic46 => "Magic46.Lib",
            LibId::Magic47 => "Magic47.Lib",
            LibId::Magic48 => "Magic48.Lib",
            LibId::Magic49 => "Magic49.Lib",
            LibId::Magic50 => "Magic50.Lib",
            LibId::Magic51 => "Magic51.Lib",
            LibId::Magic52 => "Magic52.Lib",
            LibId::Magic53 => "Magic53.Lib",
            LibId::Magic54 => "Magic54.Lib",
            LibId::Magic55 => "Magic55.Lib",
            LibId::Magic56 => "Magic56.Lib",
            LibId::Magic57 => "Magic57.Lib",
            LibId::Magic58 => "Magic58.Lib",
            LibId::Magic59 => "Magic59.Lib",
            LibId::Magic60 => "Magic60.Lib",
            LibId::Magic61 => "Magic61.Lib",
            LibId::Magic62 => "Magic62.Lib",
            LibId::Magic63 => "Magic63.Lib",
            LibId::Magic64 => "Magic64.Lib",
            LibId::Magic65 => "Magic65.Lib",
            LibId::Magic66 => "Magic66.Lib",
            LibId::Magic67 => "Magic67.Lib",
            LibId::Magic68 => "Magic68.Lib",
            LibId::Magic69 => "Magic69.Lib",
            LibId::Magic70 => "Magic70.Lib",
            LibId::Magic71 => "Magic71.Lib",
            LibId::Magic72 => "Magic72.Lib",
            LibId::Magic73 => "Magic73.Lib",
            LibId::Magic74 => "Magic74.Lib",
            LibId::Magic75 => "Magic75.Lib",
            LibId::Magic76 => "Magic76.Lib",
            LibId::Magic77 => "Magic77.Lib",
            LibId::Magic78 => "Magic78.Lib",
            LibId::Magic79 => "Magic79.Lib",
            LibId::Magic80 => "Magic80.Lib",
            LibId::Magic81 => "Magic81.Lib",
            LibId::Magic82 => "Magic82.Lib",
            LibId::Magic83 => "Magic83.Lib",
            LibId::Magic84 => "Magic84.Lib",
            LibId::Magic85 => "Magic85.Lib",
            LibId::Magic86 => "Magic86.Lib",
            LibId::Magic87 => "Magic87.Lib",
            LibId::Magic88 => "Magic88.Lib",
            LibId::Magic89 => "Magic89.Lib",
            LibId::Magic90 => "Magic90.Lib",
            LibId::Magic91 => "Magic91.Lib",
            LibId::Magic92 => "Magic92.Lib",
            LibId::Magic93 => "Magic93.Lib",
            LibId::Magic94 => "Magic94.Lib",
            LibId::Magic95 => "Magic95.Lib",
            LibId::Magic96 => "Magic96.Lib",
            LibId::Magic97 => "Magic97.Lib",
            LibId::Magic98 => "Magic98.Lib",
            LibId::Magic99 => "Magic99.Lib",
            LibId::Magic100 => "Magic100.Lib",
            LibId::Magic101 => "Magic101.Lib",
            LibId::Magic102 => "Magic102.Lib",
            LibId::Magic103 => "Magic103.Lib",
            LibId::Magic104 => "Magic104.Lib",
            LibId::Magic105 => "Magic105.Lib",
            LibId::Magic106 => "Magic106.Lib",
            LibId::Magic107 => "Magic107.Lib",
            LibId::Magic108 => "Magic108.Lib",
            LibId::Magic109 => "Magic109.Lib",
            LibId::Magic110 => "Magic110.Lib",
            LibId::Magic111 => "Magic111.Lib",
            LibId::Magic112 => "Magic112.Lib",
            LibId::Magic113 => "Magic113.Lib",
            LibId::Magic114 => "Magic114.Lib",
            LibId::Magic115 => "Magic115.Lib",
            LibId::Magic116 => "Magic116.Lib",
            LibId::Magic117 => "Magic117.Lib",
            LibId::Magic118 => "Magic118.Lib",
            LibId::Magic119 => "Magic119.Lib",
            LibId::Magic120 => "Magic120.Lib",
            LibId::Magic121 => "Magic121.Lib",
            LibId::Magic122 => "Magic122.Lib",
            LibId::Magic123 => "Magic123.Lib",
            LibId::Magic124 => "Magic124.Lib",
            LibId::Magic125 => "Magic125.Lib",
            LibId::Magic126 => "Magic126.Lib",
            LibId::Magic127 => "Magic127.Lib",
            LibId::Magic128 => "Magic128.Lib",
            LibId::Magic129 => "Magic129.Lib",
            LibId::Magic130 => "Magic130.Lib",
            LibId::Magic131 => "Magic131.Lib",
            LibId::Magic132 => "Magic132.Lib",
            LibId::Magic133 => "Magic133.Lib",
            LibId::Magic134 => "Magic134.Lib",
            LibId::Magic135 => "Magic135.Lib",
            LibId::Magic136 => "Magic136.Lib",
            LibId::Magic137 => "Magic137.Lib",
            LibId::Magic138 => "Magic138.Lib",
            LibId::Magic139 => "Magic139.Lib",
            LibId::Magic140 => "Magic140.Lib",
            LibId::Magic141 => "Magic141.Lib",
            LibId::Magic142 => "Magic142.Lib",
            LibId::Magic143 => "Magic143.Lib",
            LibId::Magic144 => "Magic144.Lib",
            LibId::Magic145 => "Magic145.Lib",
            LibId::Magic146 => "Magic146.Lib",
            LibId::Magic147 => "Magic147.Lib",
            LibId::Magic148 => "Magic148.Lib",
            LibId::Magic149 => "Magic149.Lib",
            LibId::Magic150 => "Magic150.Lib",
            LibId::Magic151 => "Magic151.Lib",
            LibId::Magic152 => "Magic152.Lib",
            LibId::Magic153 => "Magic153.Lib",
            LibId::Magic154 => "Magic154.Lib",
            LibId::Magic155 => "Magic155.Lib",
            LibId::Magic156 => "Magic156.Lib",
            LibId::Magic157 => "Magic157.Lib",
            LibId::Magic158 => "Magic158.Lib",
            LibId::Magic159 => "Magic159.Lib",
            LibId::Magic160 => "Magic160.Lib",
            LibId::Magic161 => "Magic161.Lib",
            LibId::Magic162 => "Magic162.Lib",
            LibId::Magic163 => "Magic163.Lib",
            LibId::Magic164 => "Magic164.Lib",
            LibId::Magic165 => "Magic165.Lib",
            LibId::Magic166 => "Magic166.Lib",
            LibId::Magic167 => "Magic167.Lib",
            LibId::Magic168 => "Magic168.Lib",
            LibId::Magic169 => "Magic169.Lib",
            LibId::Magic170 => "Magic170.Lib",
            LibId::Magic171 => "Magic171.Lib",
            LibId::Magic172 => "Magic172.Lib",
            LibId::Magic173 => "Magic173.Lib",
            LibId::Magic174 => "Magic174.Lib",
            LibId::Magic175 => "Magic175.Lib",
            LibId::Magic176 => "Magic176.Lib",
            LibId::Magic177 => "Magic177.Lib",
            LibId::Magic178 => "Magic178.Lib",
            LibId::Magic179 => "Magic179.Lib",
            LibId::Magic180 => "Magic180.Lib",
            LibId::Magic181 => "Magic181.Lib",
            LibId::Magic182 => "Magic182.Lib",
            LibId::Magic183 => "Magic183.Lib",
            LibId::Magic184 => "Magic184.Lib",
            LibId::Magic185 => "Magic185.Lib",
            LibId::Magic186 => "Magic186.Lib",
            LibId::Magic187 => "Magic187.Lib",
            LibId::Magic188 => "Magic188.Lib",
            LibId::Magic189 => "Magic189.Lib",
            LibId::Magic190 => "Magic190.Lib",
            LibId::Magic191 => "Magic191.Lib",
            LibId::Magic192 => "Magic192.Lib",
            LibId::Magic193 => "Magic193.Lib",
            LibId::Magic194 => "Magic194.Lib",
            LibId::Magic195 => "Magic195.Lib",
            LibId::Magic196 => "Magic196.Lib",
            LibId::Magic197 => "Magic197.Lib",
            LibId::Magic198 => "Magic198.Lib",
            LibId::Magic199 => "Magic199.Lib",
            LibId::Magic200 => "Magic200.Lib",
        }
    }
}


#pragma once

#include <cstdint>

namespace crystal_cpp_game::world::exp_table {

// 查询指定等级所需的最大经验值。
// 行为与 Rust exp_config.rs / C# Settings.LoadEXP 一致：
// 从 ./Configs/ExpList.ini 的 [Exp] 段中读取 LevelN=XXX 配置，
// 缺失的等级沿用前一级的经验值，默认起始值为 100。
std::uint64_t max_experience_for_level(std::uint16_t level) noexcept;

} // namespace crystal_cpp_game::world::exp_table

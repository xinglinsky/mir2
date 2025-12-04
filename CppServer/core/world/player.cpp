#include "world/player.hpp"
#include "world/exp_table.hpp"

#include <limits>

namespace crystal_cpp_game::world {

void Player::win_experience(std::uint64_t amount) {
    // TODO: 按 C# PlayerObject.WinExp 逻辑（组队/师徒/宠物等）逐步补全。
    // 目前仅直接进入 GainExp 的简化路径。
    gain_experience(amount);
}

void Player::gain_experience(std::uint64_t amount) {
    if (!can_gain_exp_ || amount == 0) return;

    std::uint64_t effective = amount;

    // Lover 加成：对应 C# 中的 Stats[Stat.LoverExpRatePercent]。
    if (has_lover_bonus_ && lover_exp_rate_percent_ > 0) {
        const auto bonus = (effective * static_cast<std::uint64_t>(lover_exp_rate_percent_)) / 100ULL;
        effective += bonus;
    }

    // Mentor 加成：对应 C# 中的 Stats[Stat.MentorExpRatePercent]。
    if (has_mentor_bonus_ && mentor_exp_rate_percent_ > 0) {
        const auto bonus = (effective * static_cast<std::uint64_t>(mentor_exp_rate_percent_)) / 100ULL;
        effective += bonus;
    }

    // 通用 ExpRate 百分比加成：对应 Stats[Stat.ExpRatePercent]。
    if (exp_rate_percent_ > 0) {
        const auto bonus = (effective * static_cast<std::uint64_t>(exp_rate_percent_)) / 100ULL;
        effective += bonus;
    }

    // MenteeEXP：对应 C# 中 MenteeEXP += (amount * Settings.MenteeExpBank) / 100。
    if (has_mentor_ && !is_mentor_ && mentee_exp_bank_percent_ > 0) {
        const auto bank = (effective * static_cast<std::uint64_t>(mentee_exp_bank_percent_)) / 100ULL;
        mentee_exp_ += bank;
    }

    experience_ += effective;

    // 简化版升级逻辑：当经验达到当前等级上限时循环升级，
    // 对齐 C# 中 Experience/MaxExperience 的结构。
    while (true) {
        const auto max_exp = max_experience_for_level(level_);
        if (max_exp == 0) break;
        if (experience_ < max_exp) break;

        // 消耗本级经验并升级。
        experience_ -= max_exp;
        level_up();

        if (level_ >= std::numeric_limits<std::uint16_t>::max()) {
            experience_ = 0;
            break;
        }
    }
}

void Player::level_up() {
    // TODO: 对齐 C# PlayerObject.LevelUp（刷新属性、发送 SLevelChanged 等）。
    if (level_ < std::numeric_limits<std::uint16_t>::max()) {
        ++level_;
    }
}

std::uint64_t Player::max_experience_for_level(std::uint16_t level) noexcept {
    return exp_table::max_experience_for_level(level);
}

} // namespace crystal_cpp_game::world

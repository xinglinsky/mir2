#pragma once

#include <cstdint>
#include <string>

namespace crystal_cpp_game::world {

class Player {
public:
    using Id = std::uint32_t;

    Player(Id id, std::string name)
        : id_(id), name_(std::move(name)) {}

    Id id() const noexcept { return id_; }
    const std::string& name() const noexcept { return name_; }

    std::uint16_t level() const noexcept { return level_; }
    void set_level(std::uint16_t level) noexcept { level_ = level; }

    std::uint64_t experience() const noexcept { return experience_; }
    void set_experience(std::uint64_t exp) noexcept { experience_ = exp; }

    // 查询指定等级所需的最大经验值，占位实现，后续会接 Exp 表/配置。
    static std::uint64_t max_experience_for_level(std::uint16_t level) noexcept;

    // 经验结算入口，对应 C# PlayerObject.WinExp 的最小骨架。
    void win_experience(std::uint64_t amount);

    // 实际加经验的逻辑入口，对应 C# PlayerObject.GainExp 的最小骨架。
    void gain_experience(std::uint64_t amount);

    // 升级入口，对应 C# PlayerObject.LevelUp 的最小骨架。
    void level_up();

    // 以下为经验加成相关的可配置参数，由 Buff / 关系系统驱动设置。
    void set_can_gain_experience(bool v) noexcept { can_gain_exp_ = v; }

    void set_lover_exp_rate_percent(std::int32_t v) noexcept { lover_exp_rate_percent_ = v; }
    void set_mentor_exp_rate_percent(std::int32_t v) noexcept { mentor_exp_rate_percent_ = v; }
    void set_exp_rate_percent(std::int32_t v) noexcept { exp_rate_percent_ = v; }

    void set_has_lover_bonus(bool v) noexcept { has_lover_bonus_ = v; }
    void set_has_mentor_bonus(bool v) noexcept { has_mentor_bonus_ = v; }
    void set_has_mentor(bool v) noexcept { has_mentor_ = v; }
    void set_is_mentor(bool v) noexcept { is_mentor_ = v; }

    void set_mentee_exp_bank_percent(std::int32_t v) noexcept { mentee_exp_bank_percent_ = v; }
    std::uint64_t mentee_exp() const noexcept { return mentee_exp_; }

private:
    Id id_ { 0 };
    std::string name_;
    std::uint16_t level_ { 0 };
    std::uint64_t experience_ { 0 };

    // 可获得经验与否，对应 C# 中的 CanGainExp。
    bool can_gain_exp_ { true };

    // Lover / Mentor / ExpRate 百分比加成，对应 Stats[Stat.*]。
    std::int32_t lover_exp_rate_percent_ { 0 };
    std::int32_t mentor_exp_rate_percent_ { 0 };
    std::int32_t exp_rate_percent_ { 0 };

    // 关系标志：是否有 Lover/Mentor、自己是否 Mentor。
    bool has_lover_bonus_ { false };
    bool has_mentor_bonus_ { false };
    bool has_mentor_ { false };
    bool is_mentor_ { false };

    // 对应 Settings.MenteeExpBank，用于给 Mentor 存经验。
    std::int32_t mentee_exp_bank_percent_ { 0 };
    std::uint64_t mentee_exp_ { 0 };
};

} // namespace crystal_cpp_game::world

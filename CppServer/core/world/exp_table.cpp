#include "world/exp_table.hpp"

#include <algorithm>
#include <cctype>
#include <cstdint>
#include <fstream>
#include <string>
#include <unordered_map>
#include <vector>

namespace crystal_cpp_game::world::exp_table {

namespace {

constexpr std::uint16_t MAX_LEVEL = 500;

inline void trim_inplace(std::string& s) {
    auto not_space = [](unsigned char ch) { return !std::isspace(ch); };
    s.erase(s.begin(), std::find_if(s.begin(), s.end(), not_space));
    s.erase(std::find_if(s.rbegin(), s.rend(), not_space).base(), s.end());
}

std::vector<std::uint64_t> load_exp_table(const std::string& path) {
    std::unordered_map<std::uint16_t, std::uint64_t> level_values;

    std::ifstream in(path);
    if (in) {
        std::string line;
        std::string current_section;

        while (std::getline(in, line)) {
            trim_inplace(line);
            if (line.empty()) continue;
            if (line[0] == ';' || line[0] == '#') continue;

            if (line.front() == '[' && line.back() == ']') {
                current_section.assign(line.begin() + 1, line.end() - 1);
                trim_inplace(current_section);
                continue;
            }

            auto pos = line.find('=');
            if (pos == std::string::npos) continue;
            std::string key = line.substr(0, pos);
            std::string value = line.substr(pos + 1);
            trim_inplace(key);
            trim_inplace(value);
            if (key.empty()) continue;

            if (current_section == "Exp") {
                constexpr std::string_view prefix{"Level"};
                if (key.size() > prefix.size() &&
                    std::equal(prefix.begin(), prefix.end(), key.begin())) {
                    const auto lvl_str = key.substr(prefix.size());
                    try {
                        const auto lvl = static_cast<std::uint16_t>(std::stoul(lvl_str));
                        const auto v = static_cast<std::uint64_t>(std::stoull(value));
                        level_values[lvl] = v;
                    } catch (...) {
                        // 忽略解析错误，保持默认表。
                    }
                }
            }
        }
    }

    std::vector<std::uint64_t> table;
    table.reserve(MAX_LEVEL);

    // 对齐 C# / Rust 默认起始值：100。
    std::uint64_t last = 100;
    for (std::uint16_t lvl = 1; lvl <= MAX_LEVEL; ++lvl) {
        if (auto it = level_values.find(lvl); it != level_values.end()) {
            last = it->second;
        }
        table.push_back(last);
    }

    return table;
}

const std::vector<std::uint64_t>& exp_table_instance() {
    static const std::vector<std::uint64_t> table =
        load_exp_table("./Configs/ExpList.ini");
    return table;
}

} // namespace

std::uint64_t max_experience_for_level(std::uint16_t level) noexcept {
    if (level == 0) return 0;

    const auto& table = exp_table_instance();
    if (level - 1 >= table.size()) return 0;
    return table[level - 1];
}

} // namespace crystal_cpp_game::world::exp_table

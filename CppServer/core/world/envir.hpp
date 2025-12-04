#pragma once

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

namespace crystal_cpp_game::world {

class Map;
class Player;

// Envir 对应原 C# Server/MirEnvir/Envir.cs，将逐步按文件/类 1:1 复刻。
class Envir {
public:
    // 获取主环境实例，对应 C# Envir.Main。
    static Envir& main();

    // 获取编辑环境实例，对应 C# Envir.Edit。
    static Envir& edit();

    // 简化版更新接口：后续会扩展为完整 Tick 驱动。
    void update(std::uint64_t frame_no);

    // 当前服务器逻辑时间（毫秒），对应 C# 中的 Time 字段。
    std::uint64_t time() const noexcept { return time_ms_; }

    // 是否处于运行状态，对应 C# 中的 Running 标志。
    bool running() const noexcept { return running_; }

    // 生成下一个对象 ID，对应 C# 中的 ObjectID 属性。
    std::uint32_t next_object_id() noexcept;

    // 创建并注册一张地图，对应 C# Envir 中 MapList 的管理职责。
    Map& create_map(std::int32_t index, std::string name,
                    std::int32_t width, std::int32_t height);

    // 按索引查找地图，类似 C# 中通过 Index 在 MapList 中检索 Map 的逻辑。
    Map* find_map(std::int32_t index) noexcept;

    // 创建一个玩家并注册到世界中，对应 C# Envir 中 Players 列表的管理职责。
    Player& create_player(std::string name);

    // 当前在线玩家数量，对应 C# PlayerCount 属性的简化版本。
    std::size_t player_count() const noexcept { return players_.size(); }

private:
    Envir() = default;

    bool running_ { false };
    std::uint64_t time_ms_ { 0 };

    // 对应 C# Envir 中的对象 ID 生成器（ObjectID）。
    std::uint32_t next_object_id_ { 0 };

    // 占位容器：后续将替换为完整的 Map / Player 类型。
    std::vector<std::unique_ptr<Map>> maps_;
    std::vector<std::unique_ptr<Player>> players_;
};

} // namespace crystal_cpp_game::world

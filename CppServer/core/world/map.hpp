#pragma once

#include <cstdint>
#include <string>

namespace crystal_cpp_game::world {

// Map 对应原 C# Server/MirEnvir/Map.cs，将在后续阶段按字段/逻辑逐步填充。
class Map {
public:
    using Id = std::int32_t;

    Map(Id id, std::string name, std::int32_t width, std::int32_t height);

    Id id() const noexcept { return id_; }
    const std::string& name() const noexcept { return name_; }
    std::int32_t width() const noexcept { return width_; }
    std::int32_t height() const noexcept { return height_; }

private:
    Id id_ { 0 };
    std::string name_;
    std::int32_t width_ { 0 };
    std::int32_t height_ { 0 };
};

} // namespace crystal_cpp_game::world

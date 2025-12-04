#include "world/map.hpp"

#include <utility>

namespace crystal_cpp_game::world {

Map::Map(Id id, std::string name, std::int32_t width, std::int32_t height)
    : id_(id), name_(std::move(name)), width_(width), height_(height) {}

} // namespace crystal_cpp_game::world

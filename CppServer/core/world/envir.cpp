#include "world/envir.hpp"
#include "world/map.hpp"
#include "world/player.hpp"

#include <iostream>

namespace crystal_cpp_game::world {

Envir& Envir::main() {
    static Envir instance;
    return instance;
}

Envir& Envir::edit() {
    static Envir instance;
    return instance;
}

void Envir::update(std::uint64_t frame_no) {
    // TODO: 按 C# Envir.Run/Process 行为逐步移植，这里只是占位实现。
    // 先简单地标记为运行中，并把逻辑时间推进到给定帧号对应的毫秒数。
    running_ = true;
    time_ms_ = frame_no;
}

std::uint32_t Envir::next_object_id() noexcept {
    // 简单自增，对应 C# 中的 ++_objectID 语义。
    return ++next_object_id_;
}

Map& Envir::create_map(std::int32_t index, std::string name,
                        std::int32_t width, std::int32_t height) {
    maps_.push_back(std::make_unique<Map>(index, std::move(name), width, height));
    return *maps_.back();
}

Map* Envir::find_map(std::int32_t index) noexcept {
    for (auto& m : maps_) {
        if (m && m->id() == index) {
            return m.get();
        }
    }
    return nullptr;
}

Player& Envir::create_player(std::string name) {
    auto id = next_object_id();
    players_.push_back(std::make_unique<Player>(id, std::move(name)));
    return *players_.back();
}

} // namespace crystal_cpp_game::world

#include <cstdint>

namespace crystal_cpp_net {

// 占位类型，对应未来的 GameConnection（MirConnection.cs）。
struct GameConnectionStub {
    void tick(std::uint64_t /*now_ms*/) {}
};

} // namespace crystal_cpp_net

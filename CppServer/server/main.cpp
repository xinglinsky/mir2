#include <iostream>
#include <cstdint>

#include "world/envir.hpp"

int main() {
    using crystal_cpp_game::world::Envir;

    auto& world = Envir::main();
    world.update(0);

    std::cout << "Crystal C++23 server skeleton (M0) initialized." << std::endl;
    return 0;
}

add_library(crystal_cpp_game STATIC
    world/envir.cpp
    world/map.cpp
    world/player.cpp
    world/exp_table.cpp
)

# 对外暴露 core/ 作为头文件包含根目录，例如：#include "world/envir.hpp"
target_include_directories(crystal_cpp_game PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_features(crystal_cpp_game PUBLIC cxx_std_23)

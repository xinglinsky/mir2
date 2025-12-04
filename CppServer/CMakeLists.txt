cmake_minimum_required(VERSION 3.20)
project(CrystalCppServer LANGUAGES CXX)

# 全局使用 C++23。
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# 子模块：游戏逻辑 / 基础设施 / 可执行程序。
add_subdirectory(core)
add_subdirectory(net)
add_subdirectory(db)
add_subdirectory(server)

# 汇总基础设施模块，供上层统一依赖。
add_library(crystal_cpp_infra INTERFACE)
target_link_libraries(crystal_cpp_infra INTERFACE
    crystal_cpp_infra_net
    crystal_cpp_infra_db
)

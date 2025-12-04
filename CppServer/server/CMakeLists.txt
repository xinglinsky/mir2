add_executable(crystal_cpp_server
    main.cpp
)

# 链接游戏逻辑与基础设施模块。
target_link_libraries(crystal_cpp_server PRIVATE
    crystal_cpp_game
    crystal_cpp_infra
)

target_compile_features(crystal_cpp_server PRIVATE cxx_std_23)

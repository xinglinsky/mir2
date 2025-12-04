add_library(crystal_cpp_infra_net STATIC
    connection_stub.cpp
)

# 暂时无对外头文件，后续按 MirConnection.cs 等映射补充。
target_compile_features(crystal_cpp_infra_net PUBLIC cxx_std_23)

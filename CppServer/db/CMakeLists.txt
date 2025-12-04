add_library(crystal_cpp_infra_db STATIC
    db_stub.cpp
)

# 后续会按 AccountInfo.cs / CharacterInfo.cs 等文件映射补充实现。
target_compile_features(crystal_cpp_infra_db PUBLIC cxx_std_23)

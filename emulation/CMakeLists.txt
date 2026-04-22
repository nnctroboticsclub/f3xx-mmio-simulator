find_package(cereal REQUIRED)

add_library(f3xx-mmio-simulator OBJECT emulator.cpp hal_compatiable.c)
target_link_libraries(f3xx-mmio-simulator PUBLIC cereal::cereal)

add_executable(simulator-server server.cpp)
target_link_libraries(simulator-server PRIVATE cereal::cereal)

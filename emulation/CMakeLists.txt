find_package(cereal REQUIRED)

add_library(f3xx-mmio-simulator OBJECT emulator.cpp hal_compatiable.c machine_code/interceptor.cpp)
target_link_libraries(f3xx-mmio-simulator PUBLIC cereal::cereal)
target_compile_options(f3xx-mmio-simulator PRIVATE -O0)
target_compile_options(f3xx-mmio-simulator INTERFACE "-D__ASM=if(0) asm")

add_executable(simulator-server server.cpp)
target_link_libraries(simulator-server PRIVATE cereal::cereal)

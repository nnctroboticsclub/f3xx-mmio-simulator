find_package(Newlib REQUIRED)

add_library(f3xx-mmio-simulator OBJECT emulator.cpp)
target_link_libraries(f3xx-mmio-simulator PUBLIC f3xx-mmio-simulator-rs)
target_compile_options(f3xx-mmio-simulator INTERFACE "-D__ASM=if(0) asm")

target_compile_options(f3xx-mmio-simulator INTERFACE
  -fno-exceptions
  -fno-rtti
  -fno-stack-protector
)
target_link_options(f3xx-mmio-simulator INTERFACE
  -nostdinc
  -nostdlib
  -fno-threadsafe-statics
)
target_link_libraries(f3xx-mmio-simulator INTERFACE
  Newlib::libc
)

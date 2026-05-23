find_package(Newlib REQUIRED)

add_library(f3xx-mmio-simulator OBJECT emulator.cpp)
target_link_libraries(f3xx-mmio-simulator PRIVATE Newlib::Newlib)
target_link_libraries(f3xx-mmio-simulator PUBLIC f3xx-mmio-simulator-rs)
target_compile_options(f3xx-mmio-simulator PUBLIC "-D__ASM=if(0) asm")

target_compile_options(f3xx-mmio-simulator PUBLIC
  -fno-exceptions
  -fno-rtti
  -fno-stack-protector
  -fno-threadsafe-statics
  -nostdlibinc
)
target_link_options(f3xx-mmio-simulator PUBLIC
  -nostdlib
)
target_link_libraries(f3xx-mmio-simulator INTERFACE
  Newlib::libg
  Newlib::libnosys
)

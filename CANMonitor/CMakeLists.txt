find_package(Nano REQUIRED)
find_package(F3Baremetal REQUIRED)

add_executable(CANMonitor main.cpp)
target_link_libraries(CANMonitor PUBLIC F3Baremetal::f3-baremetal Nano::Nano f3xx-mmio-simulator)
target_compile_options(CANMonitor PUBLIC -fno-threadsafe-statics)
target_compile_options(CANMonitor PUBLIC -fuse-cxa-atexit)

install(TARGETS CANMonitor
  RUNTIME DESTINATION bin
)

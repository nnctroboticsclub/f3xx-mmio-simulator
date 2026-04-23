find_package(Nano REQUIRED)
find_package(F3Baremetal REQUIRED)

add_executable(CANMonitor main.cpp)
target_link_libraries(CANMonitor PUBLIC F3Baremetal::f3-baremetal Nano::Nano)
target_link_libraries(CANMonitor PUBLIC f3xx-mmio-simulator)

install(TARGETS CANMonitor
  RUNTIME DESTINATION bin
)

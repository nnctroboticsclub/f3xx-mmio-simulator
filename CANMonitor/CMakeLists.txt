find_package(Nano REQUIRED)
find_package(F3Baremetal REQUIRED)
find_package(Newlib REQUIRED)

add_executable(CANMonitor main.cpp)
target_link_libraries(CANMonitor PUBLIC F3Baremetal::f3-baremetal Nano::Nano)
target_link_libraries(CANMonitor PUBLIC f3xx-mmio-simulator)
target_compile_options(CANMonitor PRIVATE -include ${CMAKE_CURRENT_SOURCE_DIR}/glibc_decouple.h)
target_link_options(CANMonitor PRIVATE -specs=nano.specs -specs=nosys.specs -nostdlib -Wl,--as-needed -Wl,-z,notext)
target_link_libraries(CANMonitor PRIVATE Newlib::libg Newlib::libnosys)

# Remove glibc dependencies
target_link_options(CANMonitor PRIVATE -Wl,--unresolved-symbols=ignore-all)
target_link_options(CANMonitor PRIVATE 
  -Wl,--defsym=__vprintf_chk=0 
  -Wl,--defsym=__vfprintf_chk=0 
  -Wl,--defsym=__vsnprintf_chk=0 
  -Wl,--defsym=__fprintf_chk=0 
  -Wl,--defsym=__printf_chk=0 
  -Wl,--defsym=__snprintf_chk=0
  -Wl,--defsym=__sprintf_chk=0
  -Wl,--defsym=__memcpy_chk=0
  -Wl,--defsym=__memset_chk=0
)

install(TARGETS CANMonitor
  RUNTIME DESTINATION bin
)

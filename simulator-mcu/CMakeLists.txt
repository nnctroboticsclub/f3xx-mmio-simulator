if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(CARGO_CMD cargo build --verbose)
    set(TARGET_DIR "debug")
else ()
    set(CARGO_CMD cargo build --verbose --release)
    set(TARGET_DIR "release")
endif ()

if (DEFINED ENV{CARGO_BUILD_TARGET})
    set(TARGET_DIR "$ENV{CARGO_BUILD_TARGET}/${TARGET_DIR}")
endif()

if(ENABLE_LTO)
    set(RUST_FLAGS "-Clinker-plugin-lto" "-Clinker=clang-17" "-Clink-arg=-fuse-ld=lld-17")
endif()

set(RUST_TARGET_DIR "${CMAKE_CURRENT_BINARY_DIR}")
set(F3XX_MMIO_SIMULATOR_INC "${CMAKE_CURRENT_BINARY_DIR}/include")
set(F3XX_MMIO_SIMULATOR_LIB "${RUST_TARGET_DIR}/${TARGET_DIR}/libf3xx_mmio_simulator.so")
set(F3XX_MMIO_SIMULATOR_CXX "${RUST_TARGET_DIR}/f3xx-mmio-simulator.cpp")

FILE(GLOB_RECURSE F3XX_MMIO_SIMULATOR_RS CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/src/*.rs")

add_custom_command(
    OUTPUT ${F3XX_MMIO_SIMULATOR_CXX} ${F3XX_MMIO_SIMULATOR_LIB}
    COMMAND CARGO_TARGET_DIR=${RUST_TARGET_DIR} RUSTFLAGS="${RUST_FLAGS}" ${CARGO_CMD}
    COMMAND cp ${RUST_TARGET_DIR}/cxxbridge/f3xx-mmio-simulator/src/lib.rs.cc ${F3XX_MMIO_SIMULATOR_CXX}
    COMMAND mkdir -p ${F3XX_MMIO_SIMULATOR_INC}
    COMMAND cp ${RUST_TARGET_DIR}/cxxbridge/f3xx-mmio-simulator/src/lib.rs.h ${F3XX_MMIO_SIMULATOR_INC}/f3xx-mmio-simulator.h
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    DEPENDS ${F3XX_MMIO_SIMULATOR_RS}
)

add_library(f3xx-mmio-simulator-rs-lib SHARED IMPORTED)
target_link_libraries(f3xx-mmio-simulator-rs-lib INTERFACE pthread dl)
set_target_properties(f3xx-mmio-simulator-rs-lib PROPERTIES
    IMPORTED_LOCATION "${F3XX_MMIO_SIMULATOR_LIB}"
    IMPORTED_NO_SONAME TRUE
)

add_library(f3xx-mmio-simulator-rs-rt STATIC ${F3XX_MMIO_SIMULATOR_CXX})
target_include_directories(f3xx-mmio-simulator-rs-rt PUBLIC ${F3XX_MMIO_SIMULATOR_INC})
target_link_libraries(f3xx-mmio-simulator-rs-rt PUBLIC f3xx-mmio-simulator-rs-lib)

add_library(f3xx-mmio-simulator-rs INTERFACE)
target_link_libraries(f3xx-mmio-simulator-rs INTERFACE f3xx-mmio-simulator-rs-rt)

add_test(NAME F3XX_MMIO_SIMULATOR_test
    COMMAND cargo test
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

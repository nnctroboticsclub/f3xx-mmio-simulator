if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(CARGO_CMD cargo build --verbose)
    set(TARGET_DIR "debug")
else ()
    set(CARGO_CMD cargo build --release --verbose)
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
set(F3XX_MMIO_SIMULATOR_LIB "${RUST_TARGET_DIR}/${TARGET_DIR}/libf3xx_mmio_simulator.a")
set(F3XX_MMIO_SIMULATOR_CXX "${RUST_TARGET_DIR}/f3xx-mmio-simulator.cpp")

FILE(GLOB_RECURSE F3XX_MMIO_SIMULATOR_RS CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/src/*.rs")

add_library(f3xx-mmio-simulator-rs STATIC ${F3XX_MMIO_SIMULATOR_CXX})
add_custom_command(
    OUTPUT ${F3XX_MMIO_SIMULATOR_CXX} ${F3XX_MMIO_SIMULATOR_LIB}
    COMMAND CARGO_TARGET_DIR=${RUST_TARGET_DIR} RUSTFLAGS="${RUST_FLAGS}" ${CARGO_CMD}
    COMMAND cp ${RUST_TARGET_DIR}/cxxbridge/f3xx-mmio-simulator/src/lib.rs.cc ${F3XX_MMIO_SIMULATOR_CXX}
    COMMAND mkdir -p ${F3XX_MMIO_SIMULATOR_INC}
    COMMAND cp ${RUST_TARGET_DIR}/cxxbridge/f3xx-mmio-simulator/src/lib.rs.h ${F3XX_MMIO_SIMULATOR_INC}/f3xx-mmio-simulator.h
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    DEPENDS ${F3XX_MMIO_SIMULATOR_RS}

)

target_link_libraries(f3xx-mmio-simulator-rs pthread dl ${F3XX_MMIO_SIMULATOR_LIB})
target_include_directories(f3xx-mmio-simulator-rs PUBLIC ${F3XX_MMIO_SIMULATOR_INC})

add_test(NAME F3XX_MMIO_SIMULATOR_test
    COMMAND cargo test
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

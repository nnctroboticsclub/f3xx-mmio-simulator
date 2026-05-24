if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(CARGO_CMD cargo build --verbose)
    set(TARGET_DIR "x86_64-unknown-linux-gnu/debug")
else ()
    set(CARGO_CMD cargo build --verbose --release)
    set(TARGET_DIR "x86_64-unknown-linux-gnu/release")
endif ()

set(RUST_TARGET_DIR "${CMAKE_CURRENT_BINARY_DIR}")
set(F3XX_MMIO_SIMULATOR_LIB "${RUST_TARGET_DIR}/${TARGET_DIR}/libmmio_hook.a")

FILE(GLOB_RECURSE F3XX_MMIO_SIMULATOR_RS CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/src/*.rs")

add_custom_command(
    OUTPUT ${F3XX_MMIO_SIMULATOR_LIB}
    COMMAND CARGO_TARGET_DIR=${RUST_TARGET_DIR} RUSTFLAGS="${RUST_FLAGS}" ${CARGO_CMD} --target x86_64-unknown-linux-gnu
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    DEPENDS ${F3XX_MMIO_SIMULATOR_RS}
)

add_custom_target(mmio_hook_target DEPENDS ${F3XX_MMIO_SIMULATOR_LIB})

add_library(f3xx-mmio-simulator-rs-lib STATIC IMPORTED)
set_target_properties(f3xx-mmio-simulator-rs-lib PROPERTIES
    IMPORTED_LOCATION "${F3XX_MMIO_SIMULATOR_LIB}"
)
add_dependencies(f3xx-mmio-simulator-rs-lib mmio_hook_target)

add_library(f3xx-mmio-simulator-rs INTERFACE)
target_link_libraries(f3xx-mmio-simulator-rs INTERFACE f3xx-mmio-simulator-rs-lib)

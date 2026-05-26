# Agent Notes: Project Knowledge & Nuances

This document captures critical architectural decisions, debugging findings, and implicit constraints that are not easily discoverable from the source code alone.

## 1. Process Model & IPC
- **Split Process**: The system consists of a Parent (`simulator-mcu`) and a Child (`CANMonitor`). They communicate via standard pipes (`stdin`/`stdout`).
- **READY Handshake**: The Parent **must** wait for a `READY` packet (`0xff`) from the Child before sending any data. If the Parent sends an MMIO response before the Child has registered its `SIGIO` handler, the signal will be lost, leading to a permanent hang.
- **Sequence Numbers (`seq`)**: Every `Request` has a 16-bit sequence number. The Child's `sigio_handler` **only** clears the `WAITING_FOR_DATA` flag if the `Response.seq` matches the current request. This is crucial because stale responses (from previous runs or timeouts) often linger in the pipe.

## 2. Signal Handling Complexity
- **Deadlock Avoidance (`SA_NODEFER`)**: The Child's `SIGSEGV` handler spin-waits for a response from the Parent. Because Linux normally masks a signal while its handler is running, `SIGIO` (the response trigger) would be blocked by the active `SIGSEGV` handler.
    - **Fix**: Both `SIGSEGV` and `SIGIO` are registered with `SA_NODEFER`. This allows the `SIGIO` handler to preempt the `SIGSEGV` handler's spin-loop.
- **Signal Ownership**: `F_SETOWN` must be called with the Child's own PID on `fd=0` (stdin) to enable asynchronous `SIGIO` notification for incoming pipe data.

## 3. Toolchain & Runtime Constraints
- **16-Byte Stack Alignment**: On x86_64 Linux, the stack pointer (`%rsp`) **must** be 16-byte aligned before calling any C++ function (including `main`).
    - **Symptom**: If misaligned, the C++ constructor for `std::function` (used in `CANMonitor`) triggers a `SIGSEGV` during an SSE `movaps` instruction. This appears as a crash immediately upon entering `main`.
    - **Implementation**: The Child's `_start` is written in inline assembly to explicitly align the stack before transitioning to C++ code.
- **Heap Size for `iced-x86`**: The `no_std` bridge uses a `BumpAllocator`. While the code is small, the `iced-x86` decoder lazily initializes internal lookup tables that require significant heap space (~512KB+). The heap is currently set to **1MB** to prevent OOM panics.
- **`newlib` vs `glibc`**: The Child links against `newlib` but resolves "fortified" function symbols (e.g., `__vprintf_chk`) to `0` via linker flags (`-Wl,--defsym`) to decouple it from host `glibc` requirements.

## 4. Hardware Emulation Fidelity (STM32F303x8)
- **Strict RM Compliance**: The Reference Manual (RM) states that **reserved bits MUST be preserved**. The Parent simulator enforces this by panicking if a write modification touches bits defined as reserved in the RM.
    - **Exception**: `CAN_FMR` (Filter Master Register) is relaxed because standard firmware often writes `0x1` (FINIT) directly without reading the reset value first.
- **Filter Bank Count**: The STM32F303x8 has exactly **14** bxCAN filter banks (0..13). Implementations for larger F3 variants (28 banks) will cause address range mismatches.

## 5. Build System Performance
- **Parent Binary**: `simulator-mcu` is a standard Linux host application. It **should not** use `build-std` or custom target JSON files. Using the standard `x86_64-unknown-linux-gnu` target allows it to use pre-compiled host artifacts, reducing `cargo check` time from ~30s to <1s.
- **Child Library**: `simulator` (the hook) must remain strictly `no_std` to be linkable into the bare-metal environment.

## 6. Future Considerations: Interrupts & CET
- **Intel CET Protection**: Indirect Branch Tracking (IBT) and Shadow Stack (SS) are active on modern Linux x86_64.
- **Interrupt Injection**: Manually modifying `RIP`/`RSP` in a signal handler to "jump" to an ISR will likely trigger a Shadow Stack violation (`#CP` fault) upon the ISR's `ret`. 
    - **Strategy**: Future interrupt implementations should either use an `rt_sigreturn` trampoline or disable CET via `-fcf-protection=none`.

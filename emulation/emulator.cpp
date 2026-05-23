#include <stddef.h>
#include <stdint.h>

extern "C" {
extern void (*__init_array_start[])(void) __attribute__((weak));
extern void (*__init_array_end[])(void) __attribute__((weak));

void _start() {
  if (__init_array_start && __init_array_end) {
    for (void (**p)(void) = __init_array_start; p < __init_array_end; ++p) {
      if (*p && (uintptr_t)*p != 1) (*p)();
    }
  }

  extern int main();
  main();

  // exit
  asm volatile("mov $60, %%rax\n"
               "mov $0, %%rdi\n"
               "syscall\n"
               : : : "rax", "rdi");
}

void _fini() {}
void* __dso_handle = (void*)0;

// MMIO stubs
uint64_t mmio_read(uintptr_t addr) { return 0; }
void mmio_write(uintptr_t addr, uint64_t val) {}

// SystemCoreClock
uint32_t SystemCoreClock = 8000000;

// Newlib syscall stubs (others that are not in console.hpp)
void* _sbrk(intptr_t incr) { return (void*)-1; }
int _kill(int pid, int sig) { return -1; }
int _getpid(void) { return 1; }

}

// C++ mangled name in libmmio_hook
void init_mmio_simulator();

__attribute__((constructor)) void InitEmulator() {
  init_mmio_simulator();
}

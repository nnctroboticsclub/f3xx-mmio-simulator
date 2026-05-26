#include <stddef.h>
#include <stdint.h>

extern int main();
extern "C" void init_mmio_simulator();
extern "C" uint32_t SystemCoreClock = 8000000;

// NOLINTBEGIN
extern "C" void* __dso_handle = nullptr;
extern "C" void (*__init_array_start[])(void) __attribute__((weak));
extern "C" void (*__init_array_end[])(void) __attribute__((weak));
// NOLINTEND

extern "C" void _fini() {};

extern "C" void start_c() {
  if ((__init_array_start != nullptr) && (__init_array_end != nullptr)) {
    for (void (**ptr)(void) = __init_array_start; ptr < __init_array_end;
         ++ptr) {
      if ((*ptr != nullptr) && reinterpret_cast<uintptr_t>(*ptr) != 1) {
        (*ptr)();
      }
    }
  }

  init_mmio_simulator();

  main();

  asm volatile(
      "mov $60, %%rax\n"
      "mov $0, %%rdi\n"
      "syscall\n"
      :
      :
      : "rax", "rdi");
}

asm(".global __restore_rt\n"
    "__restore_rt:\n"
    "  mov $15, %rax\n"
    "  syscall\n"
    ".global _start\n"
    "_start:\n"
    "  and $0xfffffffffffffff0, %rsp\n"
    "  call start_c\n"
    "  mov $60, %rax\n"
    "  xor %rdi, %rdi\n"
    "  syscall\n");

#include <stddef.h>
#include <stdint.h>

extern "C" {
extern void (*__init_array_start[])(void) __attribute__((weak));
extern void (*__init_array_end[])(void) __attribute__((weak));

extern int _write(int fd, const void* buf, size_t count);

int write(int fd, const void* buf, size_t count) {
  return _write(fd, buf, count);
}

long my_write(int fd, const void* buf, size_t count) {
  long ret;
  asm volatile("syscall"
               : "=a"(ret)
               : "a"(1), "D"(fd), "S"(buf), "d"(count)
               : "rcx", "r11", "memory");
  return ret;
}

extern int main();

void start_c() {
  if (__init_array_start && __init_array_end) {
    for (void (**p)(void) = __init_array_start; p < __init_array_end; ++p) {
      if (*p && (uintptr_t)*p != 1)
        (*p)();
    }
  }

  main();

  // exit
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

void _fini() {}
void* __dso_handle = (void*)0;

// MMIO stubs
uint64_t mmio_read(uintptr_t addr) {
  return 0;
}
void mmio_write(uintptr_t addr, uint64_t val) {}

// SystemCoreClock
uint32_t SystemCoreClock = 8000000;

// Newlib syscall stubs
void* _sbrk(intptr_t incr) {
  return (void*)-1;
}
int _kill(int pid, int sig) {
  return -1;
}
int _getpid(void) {
  return 1;
}
}

extern "C" void init_mmio_simulator();

__attribute__((constructor)) void InitEmulator() {
  init_mmio_simulator();
}

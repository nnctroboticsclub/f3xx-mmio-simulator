#include <cstdint>

#include "f3xx-mmio-simulator.h"

const int kSYSCLK = 8000000;

extern "C" uint32_t SystemCoreClock = kSYSCLK;
extern "C" const uint8_t AHBPrescTable[16] = {0, 0, 0, 0, 0, 0, 0, 0,
                                              1, 2, 3, 4, 6, 7, 8, 9};
extern "C" const uint8_t APBPrescTable[8] = {0, 0, 0, 0, 1, 2, 3, 4};

extern "C" void InitRCC();
extern "C" void InitVector();

__attribute__((constructor)) void InitEmulator() {
  init_mmio_simulator();

  InitRCC();
  InitVector();
}

#pragma once

#include <cstdio>
#include "mmio.hpp"

namespace mcu_emulator::mmio {
class SCBRegion : public MMIORegion {
  struct SCB_Type {
    uint32_t CPUID;     // CPUID Base Register
    uint32_t ICSR;      // Interrupt Control and State Register
    uint32_t VTOR;      // Vector Table Offset Register
    uint32_t AIRCR;     // Application Interrupt and Reset Control Register
    uint32_t SCR;       // System Control Register
    uint32_t CCR;       // Configuration Control Register
    uint8_t SHP[12U];   // System Handlers Priority Registers (4-7, 8-11, 12-15)
    uint32_t SHCSR;     // System Handler Control and State Register
    uint32_t CFSR;      // Configurable Fault Status Register
    uint32_t HFSR;      // HardFault Status Register
    uint32_t DFSR;      // Debug Fault Status Register
    uint32_t MMFAR;     // MemManage Fault Address Register
    uint32_t BFAR;      // BusFault Address Register
    uint32_t AFSR;      // Auxiliary Fault Status Register
    uint32_t PFR[2U];   // Processor Feature Register
    uint32_t DFR;       // Debug Feature Register
    uint32_t ADR;       // Auxiliary Feature Register
    uint32_t MMFR[4U];  // Memory Model Feature Register
    uint32_t ISAR[5U];  // Instruction Set Attributes Register
    uint32_t RESERVED0[5U];
    uint32_t CPACR;  // Coprocessor Access Control Register
  };

 public:
  SCBRegion(uint32_t base) : MMIORegion(base, 0x40) {}

  uint32_t read(uint32_t offset) final {
    auto value = *reinterpret_cast<uint32_t*>(
        reinterpret_cast<uint8_t*>(&scb_) + offset);
    printf("SCB: *%08x == %08x\n", offset, value);
    return value;
  }

  void write_u32(uint32_t offset, uint32_t value) final {
    // range check
    if (offset < sizeof(SCB_Type)) {
      *reinterpret_cast<uint32_t*>(reinterpret_cast<uint8_t*>(&scb_) + offset) =
          value;
      printf("SCB: %08x <- %08x\n", offset, value);
    } else {
      printf("SCB: %08x <- %08x (out of range)\n", offset, value);
    }
  }

  void reset() override {
    //
  }

 private:
  SCB_Type scb_;
};
}  // namespace mcu_emulator::mmio

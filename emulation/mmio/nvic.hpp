#pragma once

#include <cstddef>
#include <cstdint>

#include "../hardware/mcu.hpp"
#include "mmio.hpp"

namespace mcu_emulator::mmio {
class NVICRegion : public MMIORegion {
  struct RegMap {
    uint32_t ISER[8U];  //Interrupt Set Enable Register
    uint32_t RESERVED0[24U];
    uint32_t ICER[8U];  //Interrupt Clear Enable Register
    uint32_t RSERVED1[24U];
    uint32_t ISPR[8U];  //Interrupt Set Pending Register
    uint32_t RESERVED2[24U];
    uint32_t ICPR[8U];  //Interrupt Clear Pending Register
    uint32_t RESERVED3[24U];
    uint32_t IABR[8U];  //Interrupt Active bit Register
    uint32_t RESERVED4[56U];
    uint8_t IP[240U];  //Interrupt Priority Register (8Bit wide)
    uint32_t RESERVED5[644U];
    uint32_t STIR;  //Software Trigger Interrupt Register
  };

 public:
  NVICRegion(hardware::Emu& emu, uint32_t base)
      : MMIORegion(base, 0x3F0), emu_(emu) {}

  uint32_t read(uint32_t offset) final {
    if (offsetof(RegMap, ISER) <= offset and
        offset < offsetof(RegMap, RESERVED0)) {
      const auto group = (offset - offsetof(RegMap, ISER)) / sizeof(uint32_t);

      auto ret = 0;
      for (size_t i = 0; i < 32; i++) {
        ret |= emu_.nvic.IsEnabledInterrupt(i) << i;
      }

      return ret;
    }
    if (offsetof(RegMap, ICER) <= offset and
        offset < offsetof(RegMap, RESERVED0)) {
      const auto group = (offset - offsetof(RegMap, ICER)) / sizeof(uint32_t);

      auto ret = 0;
      for (size_t i = 0; i < 32; i++) {
        ret |= emu_.nvic.IsEnabledInterrupt(i) << i;
      }

      return ret;
    }
    if (offsetof(RegMap, IP) <= offset and
        offset < offsetof(RegMap, RESERVED5)) {
      return 0;
    }

    printf("NVIC Accessed on: +%08x\n", offset);
    throw std::runtime_error("Not Implemented NVIC region READ access");
  }

  void write_u32(uint32_t offset, uint32_t value) final {
    if (0 <= offset and offset < 0x80) {  // ISER
      for (size_t i = 0; i < 32; i++) {
        auto bit = bool((value >> i) & 1);
        if (not bit) {
          continue;
        }
        emu_.nvic.SetEnableInterrupt(32 * offset / 4 + i, i);
      }
      return;
    } else if (0x80 <= offset and offset < 0x100) {  // ICER
      for (size_t i = 0; i < 32; i++) {
        auto bit = bool((value >> i) & 1);
        if (not bit) {
          continue;
        }
        emu_.nvic.SetEnableInterrupt(32 * offset / 4 + i, i);
      }
      return;
    } else if (offsetof(RegMap, IP) <= offset and
               offset < offsetof(RegMap, RESERVED5)) {
      return;
    }

    printf("NVIC Accessed on: +%08x\n", offset);
    throw std::runtime_error("Not Implemented NVIC region WRITE access");
  }

  void reset() override {
    //
  }

 private:
  hardware::Emu& emu_;
};
}  // namespace mcu_emulator::mmio

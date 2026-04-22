#pragma once

#include <stdint.h>
#include <stdio.h>
#include <cstdio>

#include "../hardware/mcu.hpp"
#include "mmio.hpp"

namespace mcu_emulator::mmio {

/// @brief STM32 F3xx RCC Region
class RCCRegion : public MMIORegion {
 public:
  RCCRegion(hardware::Emu& emu) : MMIORegion(0x40021000), emu_(emu) {}

  uint32_t read(uint32_t offset) final {
    auto value = *reinterpret_cast<uint32_t*>(
        reinterpret_cast<uint8_t*>(&emu_.rcc.regs) + offset);
    // printf("RCC: *%08x == %08x\n", offset, value);
    return value;
  }

  void write_u32(uint32_t offset, uint32_t value) final {
    if (offset == 0x0) {  // CR
      auto changed_fields = emu_.rcc.regs.CR ^ value;

      if (changed_fields & 0x01000000) {  // PLL ON
        changed_fields &= ~0x01000000;
        emu_.rcc.regs.CR =
            (emu_.rcc.regs.CR & ~0x01000000) | (value & 0x01000000);

        if (value & 0x01000000) {
          emu_.rcc.regs.CR |= 0x02000000;  // PLL ready
        } else {
          emu_.rcc.regs.CR &= ~0x02000000;  // PLL not ready
        }
      }

      if (changed_fields == 0) {
        return;
      }

      printf("RCC: CR remaining fields: %08x\n", changed_fields);
    } else if (offset == 0x4) {  // CFGR
      auto changed_fields = emu_.rcc.regs.CFGR ^ value;

      if (changed_fields & (15 << 18)) {  // PLLMUL
        changed_fields &= ~(15 << 18);
        auto pllmul = (value >> 18) & 15;
        emu_.rcc.regs.CFGR =
            (emu_.rcc.regs.CFGR & ~(15 << 18)) | (pllmul << 18);
      }
      if (changed_fields & (3 << 0)) {  // SW (System clock sWitch)
        changed_fields &= ~(3 << 0);
        auto sw = (value & (3 << 0));
        emu_.rcc.regs.CFGR = (emu_.rcc.regs.CFGR & ~(3 << 0)) | (sw << 0);
        emu_.rcc.regs.CFGR = (emu_.rcc.regs.CFGR & ~(3 << 2)) | (sw << 2);
      }

      if (changed_fields & (15 << 4)) {  // HPRE
        changed_fields &= ~(15 << 4);
        emu_.rcc.regs.CFGR =
            (emu_.rcc.regs.CFGR & ~(15 << 4)) | (value & (15 << 4));
      }

      if (changed_fields & (7 << 8)) {  // PPRE1
        changed_fields &= ~(7 << 8);
        emu_.rcc.regs.CFGR =
            (emu_.rcc.regs.CFGR & ~(7 << 8)) | (value & (7 << 8));
      }

      if (changed_fields & (7 << 11)) {  // PPRE2
        changed_fields &= ~(7 << 11);
        emu_.rcc.regs.CFGR =
            (emu_.rcc.regs.CFGR & ~(7 << 11)) | (value & (7 << 11));
      }

      if (changed_fields == 0) {
        return;
      }

      printf("RCC: CFGR remaining fields: %08x\n", changed_fields);
    } else if (offset == 0x14) {  // ahb enable register
      emu_.rcc.regs.AHBENR = value;
      return;
    } else if (offset == 0x1c) {  // apb1 enable register
      emu_.rcc.regs.APB1ENR = value;
      return;
    }

    // range check
    if (offset < sizeof(hardware::RCC::RegisterMap)) {
      *reinterpret_cast<uint32_t*>(reinterpret_cast<uint8_t*>(&emu_.rcc.regs) +
                                   offset) = value;
      printf("RCC: %08x <- %08x\n", offset, value);
    } else {
      printf("RCC: %08x <- %08x (out of range)\n", offset, value);
    }
  }

  void reset() override {}

 private:
  hardware::Emu& emu_;
};
}  // namespace mcu_emulator::mmio

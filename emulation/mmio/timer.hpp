#pragma once

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include "../hardware/mcu.hpp"
#include "../hardware/timer.hpp"
#include "mmio.hpp"

namespace mcu_emulator::mmio {
template <int index>
class TIMRegion : public MMIORegion {
 public:
  TIMRegion(hardware::Emu& emu, uint32_t base)
      : MMIORegion(base, 0x400), timer_(emu), emu_(emu) {}

  uint32_t read(uint32_t offset) final {
    if (offset == 0x00) {
      auto ret = 0;
      ret |= timer_.IsEnabledPeripheral() ? 1 : 0;
      return ret;
    }
    if (offset == 0x10)
      return 0;
    if (offset == 0x0C)
      return 0;
    printf("TIM%d: Read requested on %08x\n", index, offset);
    exit(0);
  }

  void write_u32(uint32_t offset, uint32_t value) final {
    if (offset == 0x0) {  // cr1
      // printf("TIM%d: CR1 -> %08x\n", index, value);
      timer_.SetEnabledPeripheral(value & 1);
      timer_.SetPeripheralClock(emu_.rcc.APB1Clock());
      timer_.Start();
      return;
    } else if (offset == 0x28) {  // Prescaler
      // printf("TIM%d: Prescaler -> %d\n", index, value);
      timer_.SetPrescaler(value + 1);
      return;
    } else if (offset == 0x2c) {  // auto reload
      // printf("TIM%d: Auto Reload -> %d\n", index, value);
      timer_.SetAutoReloadPreload(value + 1);
      return;
    } else if (offset == 0x0C) {  // dier
      auto changed_fields = regs.dier ^ value;
      if (changed_fields & 0x00000001) {  // UIE
        auto want_to_enabled = value & 0x00000001;

        // printf("TIM%d: update Interrupt -> %d\n", index, want_to_enabled);
        timer_.SetOverflowInterruptEnabled(want_to_enabled);
        return;
      }
    } else if (offset == 0x10) {  // SR
      return;                     // noaction
    }

    printf("TIM%d: %08x <- %08x\n", index, offset, value);
    exit(1);
  }

  void reset() override {}

 private:
  struct {
    uint32_t dier;
  } regs;
  hardware::Timer timer_;

  hardware::Emu& emu_;
};
}  // namespace mcu_emulator::mmio

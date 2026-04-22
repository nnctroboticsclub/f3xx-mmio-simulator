#pragma once

#include <stdint.h>
#include <stdio.h>

namespace mcu_emulator::hardware {
struct RCC {
  struct RegisterMap {
    uint32_t CR = 0x00000083;  // RCC clock control register
    uint32_t CFGR = 0;         // RCC clock configuration register
    uint32_t CIR = 0;          // RCC clock interrupt register
    uint32_t APB2RSTR = 0;     // RCC APB2 peripheral reset register

    uint32_t APB1RSTR = 0;  // RCC APB1 peripheral reset register
    uint32_t AHBENR = 0;    // RCC AHB peripheral clock register
    uint32_t APB2ENR = 0;   // RCC APB2 peripheral clock enable register
    uint32_t APB1ENR = 0;   // RCC APB1 peripheral clock enable register

    uint32_t BDCR = 0;     // RCC Backup domain control register
    uint32_t CSR = 0;      // RCC clock control & status register
    uint32_t AHBRSTR = 0;  // RCC AHB peripheral reset register
    uint32_t CFGR2 = 0;    // RCC clock configuration register 2

    uint32_t CFGR3 = 0;  // RCC clock configuration register 3
  };

  enum ClockName { kHSI, kHSE, kPLL, kHSI_D2, kNone };

  bool PLLEnabled() { return (regs.CR & (1 << 24)) != 0; }

  void SetPLLEnabled(bool enabled) {
    if (enabled) {
      regs.CR |= (1 << 24);  // PLL ON
    } else {
      regs.CR &= ~(1 << 24);  // PLL OFF
    }
  }

  ClockName PLLSource() {
    auto raw_pll_mode = (regs.CFGR >> 16) & 1;
    return raw_pll_mode == 0 ? kHSI_D2 : kHSE;
  }

  int PLLMultiplier() {
    auto raw_pll_mul = (regs.CFGR >> 18) & 0x0F;
    return raw_pll_mul + 2;
  }

  uint32_t PLLClock() {
    auto source = GetClockFreqency(PLLSource());
    auto multiplier = PLLMultiplier();

    return source * multiplier;
  }

  uint32_t GetClockFreqency(ClockName clk) {
    switch (clk) {
      case kHSI:
        return kHSIClock;
      case kHSE:
        return kHSEClock;
      case kPLL:
        return PLLClock();
      case kHSI_D2:
        return kHSIClock / 2;
      default:
        return -1;
    }
  }

  ClockName SysclkSource() {
    auto source = (regs.CFGR >> 2) & 3;
    switch (source) {
      case 0:
        return kHSI;
      case 1:
        return kHSE;
      case 2:
        return kPLL;
      default:
        return kNone;
    }
  }

  uint32_t SystemClock() {
    auto source = SysclkSource();
    if (source == kNone) {
      return 0;
    }
    return GetClockFreqency(source);
  }

  int AHBPrescaler() {
    auto pre = (regs.CFGR >> 4) & 15;
    return (pre & 8) == 0 ? 1 : 2 << (pre & 3);
  }

  uint32_t AHBClock() { return SystemClock() / AHBPrescaler(); }

  int APB1Prescaler() {
    auto pre = (regs.CFGR >> 8) & 7;
    return (pre & 4) == 0 ? 1 : 2 << (pre & 3);
  }

  uint32_t APB1Clock() { return AHBClock() / APB1Prescaler(); }

  int APB2Prescaler() {
    auto pre = (regs.CFGR >> 11) & 7;
    return (pre & 4) == 0 ? 1 : 2 << (pre & 3);
  }

  uint32_t APB2Clock() { return AHBClock() / APB2Prescaler(); }

  void Dump() {
    printf("## RCC dump\n");
    printf("- Registers\n");
    printf("  - CR = %08x\n", regs.CR);
    printf("  - CFGR = %08x\n", regs.CFGR);
    printf("- Origin: HSI=%d HSE=%d\n", kHSIClock, kHSEClock);
    printf("- PLL: %d (x%d)\n", PLLClock(), PLLMultiplier());
    printf("- AHB=%d (/%d), APB1=%d (/%d), APB2=%d (/%d)\n", AHBClock(),
           AHBPrescaler(), APB1Clock(), APB1Prescaler(), APB2Clock(),
           APB2Prescaler());
    printf("- Sysclk: %d\n", SystemClock());
  }

 public:
  RegisterMap regs;

 private:
  const uint32_t kHSIClock = 8e6;
  const uint32_t kHSEClock = 8e6;
};

}  // namespace mcu_emulator::hardware

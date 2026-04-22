#pragma once

#include <cstdint>
#include <cstdio>
#include <stdexcept>
#include <thread>
#include "mcu.hpp"
#include "nvic.hpp"

#include "../rational.hpp"

namespace mcu_emulator::hardware {
class Timer {
 public:
  Timer(Emu& emu) : nvic(emu.nvic) {}

  void SetPeripheralClock(uint32_t peripheral_clock) {
    peripheral_clock_ = peripheral_clock;
  }
  void SetPrescaler(uint32_t prescaler) { prescaler_ = prescaler; }
  void SetAutoReloadPreload(uint32_t auto_reload_preload) {
    auto_reload_preload_ = auto_reload_preload;
  }

  void SetEnabledPeripheral(bool enabled) { peripheral_enabled_ = enabled; }
  void SetOverflowInterruptEnabled(bool enabled) {
    overflow_interrupt_enabled_ = enabled;
  }

  auto IsEnabledPeripheral() const { return peripheral_enabled_; }

  void Start() {
    std::thread thread([this]() {
      using namespace std::chrono_literals;

      while (not peripheral_enabled_ or not overflow_interrupt_enabled_) {
        std::this_thread::sleep_for(100ms);
      }
      printf("TIM: Start\n");

      while (true) {
        if (overflow_interrupt_enabled_) {
          nvic.FireInterrupt(IRQn{IRQn_Type::TIM6_DAC1_IRQn});
        }

        auto count_period_ns =
            double(Rational<uint32_t>{} * prescaler_ * uint(1e9) / uint(40e6));
        auto count_period = std::chrono::nanoseconds(int(count_period_ns));
        auto wait = count_period * auto_reload_preload_;
        if (wait.count() == 0) {
          printf("-----------\n");
          printf("- count_period_ns: %f\n", count_period_ns);
          printf("- count_period: %ld\n", count_period.count());
          printf("- wait: %ld\n", wait.count());
          printf("- prescaler: %d\n", prescaler_);
          printf("- auto_reload_preload: %d\n", auto_reload_preload_);
          printf("- peripheral_clock: %d\n", peripheral_clock_);
          printf("-----------\n");
          *(int*)0 = 0;
        }
        std::this_thread::sleep_for(wait);
      }
    });

    thread.detach();
  }

 private:
  NVIC& nvic;

  uint32_t peripheral_clock_ = 0;
  uint32_t prescaler_ = 0;
  uint32_t auto_reload_preload_ = 0;

  bool peripheral_enabled_ = false;
  bool overflow_interrupt_enabled_ = false;
};
}  // namespace mcu_emulator::hardware

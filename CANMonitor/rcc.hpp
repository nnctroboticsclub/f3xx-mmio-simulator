#pragma once

#include <f3/peripherals/rcc.hpp>
#include "event_log.hpp"

namespace CANMonitor {
using namespace stm32f3::rcc;

using BaremetalRCC =
    RCCConfig<ClockOrigin{.HSI = 8000000, .HSE = 8000000},
              PLLConfig<PLLSource_HSI_D2, 10>,
              SystemClockConfig<SystemClockSource::kPLL>,
              BusClockConfig<AHBPrescaler::kDiv1, APB1Prescaler::kDiv1,
                             APB2Prescaler::kDiv1>>;
static_assert(BaremetalRCC::GetAPB1Clock() == 40e6);
static_assert(BaremetalRCC::GetAPB2Clock() == 40e6);
static_assert(BaremetalRCC::GetAHBClock() == 40e6);

bool rcc_initialized = false;

}  // namespace CANMonitor

extern "C" uint32_t SystemCoreClock __attribute__((weak));

namespace stm32 {
extern "C" void InitRCC() {
  CANMonitor::BaremetalRCC::ApplyConfig();

  SystemCoreClock = CANMonitor::BaremetalRCC::GetSystemClock();

  CANMonitor::rcc_initialized = true;
}
}  // namespace stm32
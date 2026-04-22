#pragma once

#include <cstdint>

#include <f3/peripherals/basic_timer.hpp>

#include "rcc.hpp"

using Timer = stm32f3::basic_timer::BasicTimer<6>;

template <stm32f3::basic_timer::TimerHandler Handler>
void InitTimer() {
  Timer::Init<1000, CANMonitor::BaremetalRCC, Handler>();
}

void WaitMS(uint32_t ms) {
  for (volatile uint32_t i = 0; i < ms * 8000; i++)
    ;
}
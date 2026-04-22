#pragma once

#include "../emulator_core.hpp"
#include "nvic.hpp"
#include "rcc.hpp"

namespace mcu_emulator::hardware {

struct Emu {
  EmulationNetwork network;
  RCC rcc;
  NVIC nvic;
};
}  // namespace mcu_emulator::hardware

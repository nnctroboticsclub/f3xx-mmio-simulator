#pragma once

#include <cstdint>

#include "mmio.hpp"

namespace mcu_emulator::mmio {
template <int index>
class GPIORegion : public MMIORegion {
  struct GPIO_TypeDef {
    uint32_t MODER;    // GPIO port mode register,
    uint32_t OTYPER;   // GPIO port output type register,
    uint32_t OSPEEDR;  // GPIO port output speed register,
    uint32_t PUPDR;    // GPIO port pull-up/pull-down register,

    uint32_t IDR;   // GPIO port input data register,
    uint32_t ODR;   // GPIO port output data register,
    uint32_t BSRR;  // GPIO port bit set/reset register,
    uint32_t LCKR;  // GPIO port configuration lock register,

    uint32_t AFR[2];  // GPIO alternate function registers,
    uint32_t BRR;     // GPIO bit reset register,
  };
  GPIO_TypeDef port;

 public:
  GPIORegion() : MMIORegion(0x48000000 + 0x400 * index, 0x30) {}

  uint32_t read(uint32_t offset) final {
    auto value = *reinterpret_cast<uint32_t*>(
        reinterpret_cast<uint8_t*>(&port) + offset);
    if (offset == 0x00) {  // IDR
      // printf("GPIO%d: Sampling\n", index);
    }
    return value;
  }

  void write_u32(uint32_t offset, uint32_t value) final {
    if (offset == 0x14) {  // ODR
      // printf("GPIO%d: %08x\n", index, value);
      port.IDR = value;  // Simulate read from IDR
      return;
    }

    // range check
    if (offset < sizeof(GPIO_TypeDef)) {
      *reinterpret_cast<uint32_t*>(reinterpret_cast<uint8_t*>(&port) + offset) =
          value;
      // printf("GPIO%d: %08x <- %08x\n", index, offset, value);
    } else {
      // printf("GPIO%d: %08x <- %08x (out of range)\n", index, offset, value);
    }
  }

  void reset() override {}
};
}  // namespace mcu_emulator::mmio

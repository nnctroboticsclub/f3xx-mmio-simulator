#pragma once

#include <cstddef>
#include "mmio.hpp"

namespace mcu_emulator::mmio {
// TODO: Implement DMA
class DMARegion : public MMIORegion {
  struct DMA_Type {
    uint32_t ISR;   // DMA interrupt status register,
    uint32_t IFCR;  // DMA interrupt flag clear register,

    struct Channel {
      uint32_t CCR;    // DMA channel x configuration register
      uint32_t CNDTR;  // DMA channel x number of data register
      uint32_t CPAR;   // DMA channel x peripheral address register
      uint32_t CMAR;   // DMA channel x memory address register
      uint32_t res;
    };
    Channel ch[7];
  };
  static_assert(offsetof(DMA_Type, ch[0]) == 0x08);
  static_assert(offsetof(DMA_Type, ch[1]) == 0x1C);
  static_assert(offsetof(DMA_Type, ch[2]) == 0x30);
  static_assert(offsetof(DMA_Type, ch[3]) == 0x44);
  static_assert(offsetof(DMA_Type, ch[4]) == 0x58);
  static_assert(offsetof(DMA_Type, ch[5]) == 0x6C);
  static_assert(offsetof(DMA_Type, ch[6]) == 0x80);

 public:
  DMARegion(uint32_t base) : MMIORegion(base, 0x400) {}

  uint32_t read(uint32_t offset) final {
    auto value = *reinterpret_cast<uint32_t*>(
        reinterpret_cast<uint8_t*>(&scb_) + offset);
    // printf("DMA: *%08x == %08x\n", offset, value);
    return value;
  }

  void write_u32(uint32_t offset, uint32_t value) final {
    *reinterpret_cast<uint32_t*>(reinterpret_cast<uint8_t*>(&scb_) + offset) =
        value;

    // printf("DMA: %08x <- %08x\n", offset, value);
  }

  void reset() override {
    //
  }

 private:
  DMA_Type scb_;
};
}  // namespace mcu_emulator::mmio

#pragma once

#include <cstdio>
#include "mmio.hpp"
namespace mcu_emulator::mmio {
class FlashRegion : public MMIORegion {
  struct FLASH_TypeDef {
    uint32_t ACR;      // FLASH access control register
    uint32_t KEYR;     // FLASH key register
    uint32_t OPTKEYR;  // FLASH option key register
    uint32_t SR;       // FLASH status register
    uint32_t CR;       // FLASH control register
    uint32_t AR;       // FLASH address register
    uint32_t RESERVED;
    uint32_t OBR;   // FLASH Option byte register
    uint32_t WRPR;  // FLASH Write register
  };

 public:
  FlashRegion(uint32_t base) : MMIORegion(base, 0x400) {}

  uint32_t read(uint32_t offset) final {
    auto value = *reinterpret_cast<uint32_t*>(
        reinterpret_cast<uint8_t*>(&flash_) + offset);
    return value;
  }

  void write_u32(uint32_t offset, uint32_t value) final {
    // range check
    if (offset < sizeof(FLASH_TypeDef)) {
      *reinterpret_cast<uint32_t*>(reinterpret_cast<uint8_t*>(&flash_) +
                                   offset) = value;
      printf("FLASH: %08x <- %08x\n", offset, value);
    } else {
      printf("FLASH: %08x <- %08x (out of range)\n", offset, value);
    }
  }

  void reset() override {}

 private:
  FLASH_TypeDef flash_;
};
}  // namespace mcu_emulator::mmio

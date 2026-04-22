#pragma once

#include <sys/mman.h>
#include <cstdint>

namespace mcu_emulator::mmio {
/// @brief 4KiB MMIO memory region
class MMIORegion {
 public:
  MMIORegion(uint32_t start, uint32_t len = 0x1000) : start_(start), len_(len) {
    mmap((void*)start_, len, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  }
  virtual ~MMIORegion() = default;

  uint32_t Start() const { return start_; }
  uint32_t End() const { return start_ + len_; }

  bool Contains(uint32_t address) const {
    auto end = End();
    return start_ <= address && address < end;
  }

  virtual uint32_t read(uint32_t offset) = 0;
  virtual void write_u32(uint32_t offset, uint32_t value) = 0;
  virtual void write_u8(uint32_t offset, uint32_t value) {
    auto original_value = read(offset & ~0x3);
    original_value &= ~(0xFF << ((offset & 0x3) * 8));
    original_value |= (value & 0xFF) << ((offset & 0x3) * 8);
    write_u32(offset & ~0x3, original_value);
  }
  virtual void reset() = 0;

 private:
  uint32_t start_;
  uint32_t len_;
};

}  // namespace mcu_emulator::mmio

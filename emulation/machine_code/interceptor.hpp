#pragma once

#include <sys/ucontext.h>
#include <cstdint>
#include <optional>

#include "../mmio/mmio.hpp"
#include "modrm.hpp"

namespace mcu_emulator::machine_code {
class MachineCodeInterceptor {
 private:
  [[nodiscard]] auto TakePrefixREX() -> bool;
  [[nodiscard]] auto TakePrefix16Bit() -> bool;
  [[nodiscard]] auto TakeSIB(mcu_emulator::machine_code::ModRM& modrm) -> bool;
  [[nodiscard]] auto TakeModRM()
      -> std::optional<mcu_emulator::machine_code::ModRM>;

 public:
  void ShowCode() const;

  MachineCodeInterceptor(uintptr_t pc, mcu_emulator::mmio::MMIORegion* region,
                         uint32_t offset, mcontext_t* mcontext);

  [[nodiscard]]
  auto Intercept() -> uintptr_t;

 private:
  union {
    uintptr_t pc;
    uint8_t* code_buf;
  };
  mcu_emulator::mmio::MMIORegion* region;
  uint32_t offset;
  mcontext_t* mcontext;
};
}  // namespace mcu_emulator::machine_code
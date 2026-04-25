#pragma once

#include "reg.hpp"

namespace mcu_emulator::machine_code {
enum class ModRM_Mod : uint8_t {
  MOD_NO_DISP = 0b00,
  MOD_DISP8 = 0b01,
  MOD_DISP32 = 0b10,
  MOD_REG = 0b11,
};

/// @brief ModRM byte decoder
class ModRM {
 public:
  explicit ModRM(uint8_t byte)
      : mod_(static_cast<ModRM_Mod>(static_cast<uint8_t>(byte >> 6U) & 0b11U)),
        reg_(Reg::FromIndex((byte >> 3U) & 0b111U).value_or(Reg::AX())),
        rm_(Reg::FromIndex(byte & 0b111U).value_or(Reg::AX())) {}

  [[nodiscard]] auto GetMod() const -> ModRM_Mod { return mod_; }
  [[nodiscard]] auto GetReg() const -> Reg { return reg_; }
  [[nodiscard]] auto GetRm() const -> Reg { return rm_; }

 private:
  ModRM_Mod mod_;
  Reg reg_;
  Reg rm_;
};

}  // namespace mcu_emulator::machine_code

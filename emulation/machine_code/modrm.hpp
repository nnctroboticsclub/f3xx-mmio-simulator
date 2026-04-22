#pragma once

#include "reg.hpp"

namespace mcu_emulator::machine_code {
/// @brief ModRM byte decoder
class ModRM {
 public:
  enum class Mod {
    MOD_NO_DISP = 0b00,
    MOD_DISP8 = 0b01,
    MOD_DISP32 = 0b10,
    MOD_REG = 0b11,
  };

 public:
  ModRM(uint8_t byte)
      : mod_(static_cast<Mod>((byte >> 6) & 0b11)),
        reg_(Reg::FromIndex((byte >> 3) & 0b111).value_or(Reg::AX())),
        rm_(Reg::FromIndex(byte & 0b111).value_or(Reg::AX())) {}

  Mod GetMod() const { return mod_; }
  Reg GetReg() const { return reg_; }
  Reg GetRm() const { return rm_; }

 private:
  Mod mod_;
  Reg reg_;
  Reg rm_;
};

}  // namespace mcu_emulator::machine_code

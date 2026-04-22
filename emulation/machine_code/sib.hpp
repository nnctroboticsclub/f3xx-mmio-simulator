#pragma once

#include "reg.hpp"

namespace mcu_emulator::machine_code {
/// @brief SIB byte decoder
class SIB {
 public:
  enum class Scaler {
    kX1 = 0b00,
    kX2 = 0b01,
    kX4 = 0b10,
    kX8 = 0b11,
  };

 public:
  SIB(uint8_t byte)
      : mod_(static_cast<Scaler>((byte >> 6) & 0b11)),
        reg_(Reg::FromIndex((byte >> 3) & 0b111).value_or(Reg::AX())),
        rm_(Reg::FromIndex(byte & 0b111).value_or(Reg::AX())) {}

  Scaler Scale() const { return mod_; }
  Reg Index() const { return reg_; }
  Reg Base() const { return rm_; }

 private:
  Scaler mod_;
  Reg reg_;
  Reg rm_;
};
}  // namespace mcu_emulator::machine_code

#pragma once

#include <sys/ucontext.h>

#include <optional>
#include <string>

namespace mcu_emulator::machine_code {
class Reg {
  enum class RegId {
    AX = 0b000,
    CX = 0b001,
    DX = 0b010,
    BX = 0b011,
    SP = 0b100,
    BP = 0b101,
    SI = 0b110,
    DI = 0b111
  };

  Reg(RegId id) : id_(id) {}

 public:
  operator std::string() {
    static const char* strings[] = {"ax", "cx", "dx", "bx",
                                    "sp", "bp", "si", "di"};

    if (id_ < RegId::AX || id_ > RegId::DI) {
      return "??";
    }
    return std::string() + "\x1b[1;34m" + strings[static_cast<int>(id_)] +
           "\x1b[m";
  }

  std::string ToString() { return std::string(*this); }

  size_t Index() {
    switch (id_) {
      case RegId::AX:
        return REG_RAX;
        break;
      case RegId::CX:
        return REG_RCX;
        break;
      case RegId::DX:
        return REG_RDX;
        break;
      case RegId::BX:
        return REG_RBX;
        break;

      case RegId::SP:
        return REG_RSP;
        break;
      case RegId::BP:
        return REG_RBP;
        break;
      case RegId::SI:
        return REG_RSI;
        break;
      case RegId::DI:
        return REG_RDI;
        break;

      default:
        return -1;
        break;
    }
  }

  size_t Encode() { return static_cast<size_t>(id_); }

  void Write(mcontext_t* mcontext, uint64_t value) {
    auto index = Index();
    if (index != -1) {
      mcontext->gregs[index] = value;
    } else {
      printf("Invalid register index\n");
    }
  }

  uint64_t Read(mcontext_t* mcontext) {
    auto index = Index();
    if (index != -1) {
      return mcontext->gregs[index];
    } else {
      printf("Invalid register index\n");
      return 0;
    }
  }

 public:
  static std::optional<Reg> FromIndex(int index) {
    if (index < 0 || index > 7) {
      return std::nullopt;
    }

    auto reg_id = static_cast<RegId>(index);
    return Reg(reg_id);
  }

  static Reg AX() { return Reg(RegId::AX); }
  static Reg CX() { return Reg(RegId::CX); }
  static Reg DX() { return Reg(RegId::DX); }
  static Reg BX() { return Reg(RegId::BX); }
  static Reg SP() { return Reg(RegId::SP); }
  static Reg BP() { return Reg(RegId::BP); }
  static Reg SI() { return Reg(RegId::SI); }
  static Reg DI() { return Reg(RegId::DI); }

 private:
  RegId id_;
};
}  // namespace mcu_emulator::machine_code

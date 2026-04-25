#pragma once

#include <sys/ucontext.h>

#include <cstdint>
#include <optional>
#include <string>

namespace mcu_emulator::machine_code {
enum class RegId : uint8_t {
  AX = 0b000,
  CX = 0b001,
  DX = 0b010,
  BX = 0b011,
  SP = 0b100,
  BP = 0b101,
  SI = 0b110,
  DI = 0b111
};

// NOLINTNEXTLINE
static const char* kRegNames[] = {"ax", "cx", "dx", "bx",
                                  "sp", "bp", "si", "di"};

const auto kRegCount = sizeof(kRegNames) / sizeof(kRegNames[0]);

class Reg {
  explicit Reg(RegId reg_id) : id_(reg_id) {}

 public:
  explicit operator std::string() {
    if (id_ < RegId::AX || id_ > RegId::DI) {
      return "??";  // NOLINT
    }
    // NOLINTNEXTLINE
    return std::string() + "\x1b[1;34m" + kRegNames[static_cast<int>(id_)] +
           "\x1b[m";
  }

  auto ToString() -> std::string { return std::string(*this); }

  auto Index() -> size_t {
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

  auto Encode() -> size_t { return static_cast<size_t>(id_); }

  void Write(mcontext_t* mcontext, int64_t value) {
    auto index = Index();
    if (index != -1) {
      mcontext->gregs[index] = value;
    } else {
      printf("Invalid register index\n");
    }
  }

  auto Read(mcontext_t* mcontext) -> int64_t {
    auto index = Index();
    if (index == -1) {
      printf("Invalid register index\n");
      return 0;
    }

    return mcontext->gregs[index];
  }

  static auto FromIndex(int index) -> std::optional<Reg> {
    if (index < 0 || index >= kRegCount) {
      return std::nullopt;
    }

    auto reg_id = static_cast<RegId>(index);
    return Reg(reg_id);
  }

  static auto AX() -> Reg { return Reg{RegId::AX}; }
  static auto CX() -> Reg { return Reg{RegId::CX}; }
  static auto DX() -> Reg { return Reg{RegId::DX}; }
  static auto BX() -> Reg { return Reg{RegId::BX}; }
  static auto SP() -> Reg { return Reg{RegId::SP}; }
  static auto BP() -> Reg { return Reg{RegId::BP}; }
  static auto SI() -> Reg { return Reg{RegId::SI}; }
  static auto DI() -> Reg { return Reg{RegId::DI}; }

 private:
  RegId id_;
};
}  // namespace mcu_emulator::machine_code

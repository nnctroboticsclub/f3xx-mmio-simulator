#include "interceptor.hpp"

#include "sib.hpp"

namespace mcu_emulator::machine_code {
[[nodiscard]] auto MachineCodeInterceptor::TakePrefixREX() -> bool {
  if ((*code_buf & 0xF0) == 0x40) {  // REX prefix
    uint8_t rex = *code_buf++;
  }
  return true;
}
[[nodiscard]] auto MachineCodeInterceptor::TakePrefix16Bit() -> bool {
  if (*code_buf == 0x66) {  // 16-bit operand prefix
    code_buf += 1;
  }
  return true;
}
[[nodiscard]] auto MachineCodeInterceptor::TakeSIB(
    mcu_emulator::machine_code::ModRM& modrm) -> bool {
  SIB sib(*code_buf++);
  if (sib.Base().Encode() == 0b101 and
      modrm.GetMod() == ModRM_Mod::MOD_NO_DISP) {
    code_buf += 4;
  }
  return true;
}
[[nodiscard]] auto MachineCodeInterceptor::TakeModRM()
    -> std::optional<mcu_emulator::machine_code::ModRM> {
  ModRM modrm(*code_buf++);

  if (modrm.GetMod() != ModRM_Mod::MOD_REG) {
    auto r_m = modrm.GetRm();

    if (r_m.Encode() == 0b101) {
      printf("Unhandled instruction: 8Bh with rm=0b101[rbp] (disp32)\n");
      return std::nullopt;
    }
    if (r_m.Encode() == 0b100) {
      if (!TakeSIB(modrm)) {
        return std::nullopt;
      }
    }
  }

  if (modrm.GetMod() == ModRM_Mod::MOD_DISP8) {
    code_buf += 1;
  } else if (modrm.GetMod() == ModRM_Mod::MOD_DISP32) {
    code_buf += 4;
  }
  return std::make_optional(modrm);
}

void MachineCodeInterceptor::ShowCode() const {
  printf("%016llx: ", pc);
  for (int i = 0; i < 15; i++) {
    auto value = *(code_buf + i);
    printf("%02x", value);
  }
  printf("\n");
}

MachineCodeInterceptor::MachineCodeInterceptor(
    uintptr_t pc, mcu_emulator::mmio::MMIORegion* region, uint32_t offset,
    mcontext_t* mcontext)
    : pc(pc), region(region), offset(offset), mcontext(mcontext) {}

[[nodiscard]]
auto MachineCodeInterceptor::Intercept() -> uintptr_t {
  constexpr int kRegEFL = 17;

  (void)TakePrefixREX();
  (void)TakePrefix16Bit();

  if (*code_buf == 0x8B) {  // MOV Ev, Gv
    code_buf++;

    auto reg = TakeModRM();
    if (!reg) {
      goto fail;
    }

    auto value = region->read(offset);
    reg->GetReg().Write(mcontext, value);
    return pc;
  }
  if (code_buf[0] == 0x89) {  // Mov Gv, Ev
    code_buf++;

    auto reg = TakeModRM();
    if (!reg) {
      goto fail;
    }

    auto value = static_cast<uint32_t>(reg->GetReg().Read(mcontext));
    region->write_u32(offset, value);
    return pc;
  }
  if (code_buf[0] == 0xc7) {  // Mov Gv, Ev
    code_buf++;

    auto reg = TakeModRM();
    if (!reg) {
      goto fail;
    }

    auto imm = *reinterpret_cast<uint32_t*>(code_buf);
    code_buf += 4;

    region->write_u32(offset, imm);
    return pc;
  }
  if (code_buf[0] == 0x88) {  // mov rm <-- reg (8bit)
    code_buf++;

    auto reg = TakeModRM();
    if (!reg) {
      goto fail;
    }

    auto value = reg->GetReg().Read(mcontext) & 0xFFU;
    region->write_u8(offset, value);

    return pc;
  }
  if (code_buf[0] == 0x0f and code_buf[1] == 0xb7) {  // reg32 <-- rm8
    code_buf += 2;

    auto reg = TakeModRM();
    if (!reg) {
      goto fail;
    }

    reg->GetReg().Write(mcontext, region->read(offset));
    return pc;
  }
  if (code_buf[0] == 0x83) {
    code_buf++;

    auto modrm = TakeModRM();
    if (!modrm) {
      goto fail;
    }

    uint8_t imm = *code_buf++;

    uint32_t value = region->read(offset);
    auto mode = modrm->GetReg();
    if (mode.Encode() == 0b001) {  // OR
      value |= imm;
    } else if (mode.Encode() == 0b100) {  // AND
      value &= imm;
    } else if (mode.Encode() == 0b110) {  // XOR
      value ^= imm;
    } else {
      printf("Unhandled instruction: 83h with mod=0b00 and reg=0b%03b\n",
             mode.Encode());
      goto fail;
    }

    region->write_u32(offset, value);
    return pc;
  }
  if (code_buf[0] == 0x81) {
    code_buf++;

    auto modrm = TakeModRM();
    if (!modrm) {
      goto fail;
    }

    uint32_t imm = *reinterpret_cast<uint32_t*>(code_buf);
    code_buf += 4;

    auto mode = modrm->GetReg();
    uint32_t value = region->read(offset);

    if (mode.Encode() == 0b001) {
      value |= imm;
    } else if (mode.Encode() == 0b100) {  // AND
      value &= imm;
    } else {
      printf("Unhandled instruction: 81h with mod=0b00 and reg=0b%03b\n",
             mode.Encode());
      goto fail;
    }

    region->write_u32(offset, value);

    return pc;
  }
  if (code_buf[0] == 0x0F and code_buf[1] == 0xBA) {
    code_buf += 2;

    auto modrm = TakeModRM();

    uint8_t imm = *code_buf++;

    uint32_t value = region->read(offset);
    bool bit = (value >> imm) & 1;

    auto mode = modrm->GetReg();
    if (mode.Encode() == 0b100) {
      // BT
      mcontext->gregs[kRegEFL] =
          (mcontext->gregs[kRegEFL] & ~0x40) | (bit << 6);
      return pc;
    } else {
      printf("Unhandled instruction: 0FBAh with rm=0b%03b (mod=0b00)\n",
             mode.Encode());
      goto fail;
    }
  }
  if (code_buf[0] == 0xf7) {
    code_buf++;

    auto modrm = TakeModRM();
    if (!modrm) {
      goto fail;
    }

    uint32_t value = region->read(offset);

    auto mode = modrm->GetReg();
    if (mode.Encode() == 0b000) {  // TEST r/m32, imm32
      uint32_t imm = *reinterpret_cast<uint32_t*>(code_buf);
      code_buf += 4;
      uint32_t result = value & imm;

      mcontext->gregs[kRegEFL] = (mcontext->gregs[kRegEFL] & ~0xC7) |
                                 ((result == 0) << 6) | ((result >> 31) << 7);
      return pc;
    } else {
      printf("Unhandled instruction: F7h with mod=0b00 and reg=0b%03b\n",
             mode.Encode());
      goto fail;
    }
  }
  if (code_buf[0] == 0xc6) {
    code_buf++;

    auto modrm = TakeModRM();
    if (!modrm) {
      goto fail;
    }

    uint8_t imm = *code_buf++;
    region->write_u8(offset, imm);
    return pc;
  }

fail:
  printf(
      "\x1b[1;31m======\x1b[m Unhandled instruction "
      "\x1b[1;31m======\x1b[m\n");
  printf("Attempting to access %p failed\n", region->Start() + offset);
  ShowCode();
  printf("Signal handler finished\n");
  abort();
}

}  // namespace mcu_emulator::machine_code
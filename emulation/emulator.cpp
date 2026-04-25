#include <cerrno>
#include <csignal>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstdlib>

#include <sys/mman.h>
#include <sys/ptrace.h>
#include <sys/signal.h>
#include <sys/ucontext.h>
#include <ucontext.h>
#include <unistd.h>

#include <memory>
#include <optional>
#include <vector>
#include "hardware/bridge.hpp"
#include "hardware/mcu.hpp"
#include "machine_code/modrm.hpp"
#include "machine_code/reg.hpp"
#include "machine_code/sib.hpp"
#include "mmio/adc.hpp"
#include "mmio/can.hpp"
#include "mmio/dma.hpp"
#include "mmio/flash.hpp"
#include "mmio/gpio.hpp"
#include "mmio/mmio.hpp"
#include "mmio/nvic.hpp"
#include "mmio/rcc.hpp"
#include "mmio/scb.hpp"
#include "mmio/timer.hpp"
#include "mmio/usart.hpp"

namespace {
using MMIORegions =
    std::vector<std::shared_ptr<mcu_emulator::mmio::MMIORegion>>;

MMIORegions regions;

auto LookupRegion(uint32_t address)
    -> std::shared_ptr<mcu_emulator::mmio::MMIORegion> {
  for (auto& r : regions) {
    if (r->Contains(address)) {
      return r;
    }
  }
  return nullptr;
}

[[noreturn]] void ReraiseAsNativeSegfault(int sig) {
  struct sigaction default_action{};
  default_action.sa_handler = SIG_DFL;
  sigemptyset(&default_action.sa_mask);
  default_action.sa_flags = 0;
  sigaction(sig, &default_action, nullptr);
  (void)raise(sig);

  _exit(128 + sig);
}

class MachineCodeInterceptor {
 private:
  [[nodiscard]] auto TakePrefixREX() -> bool {
    if ((*code_buf & 0xF0) == 0x40) {  // REX prefix
      uint8_t rex = *code_buf++;
    }
    return true;
  }
  [[nodiscard]] auto TakePrefix16Bit() -> bool {
    if (*code_buf == 0x66) {  // 16-bit operand prefix
      code_buf += 1;
    }
    return true;
  }
  [[nodiscard]] auto TakeSIB(mcu_emulator::machine_code::ModRM& modrm) -> bool {
    using mcu_emulator::machine_code::ModRM;
    using mcu_emulator::machine_code::SIB;
    SIB sib(*code_buf++);
    if (sib.Base().Encode() == 0b101 and
        modrm.GetMod() == ModRM::Mod::MOD_NO_DISP) {
      code_buf += 4;
    }
    return true;
  }
  [[nodiscard]] auto TakeModRM()
      -> std::optional<mcu_emulator::machine_code::ModRM> {
    using mcu_emulator::machine_code::ModRM;
    ModRM modrm(*code_buf++);

    if (modrm.GetMod() != ModRM::Mod::MOD_REG) {
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

    if (modrm.GetMod() == ModRM::Mod::MOD_DISP8) {
      code_buf += 1;
    } else if (modrm.GetMod() == ModRM::Mod::MOD_DISP32) {
      code_buf += 4;
    }
    return std::make_optional(modrm);
  }

  [[nodiscard]] bool TakeIf(uint8_t expected) {
    if (*code_buf == expected) {
      code_buf++;
      return true;
    }
    return false;
  }

 public:
  void ShowCode() {
    printf("%016llx: ", pc);
    for (int i = 0; i < 15; i++) {
      auto value = *(code_buf + i);
      printf("%02x", value);
    }
    printf("\n");
  }

  MachineCodeInterceptor(uintptr_t pc, mcu_emulator::mmio::MMIORegion* region,
                         uint32_t offset, mcontext_t* mcontext)
      : pc(pc), region(region), offset(offset), mcontext(mcontext) {}

  [[nodiscard]]
  uintptr_t Intercept() {
    constexpr int kRegEFL = 17;
    using mcu_emulator::machine_code::ModRM;
    using mcu_emulator::machine_code::SIB;

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

 private:
  union {
    uintptr_t pc;
    uint8_t* code_buf;
  };
  mcu_emulator::mmio::MMIORegion* region;
  uint32_t offset;
  mcontext_t* mcontext;
};

static void handler(int sig, siginfo_t* si, void* platform) {
  auto address = si->si_addr;
  auto address_int = reinterpret_cast<uintptr_t>(address);

  auto region = LookupRegion(address_int);
  if (!region) {
    printf(
        "\x1b[1;31m======\x1b[m Segmentation Fault. "
        "\x1b[1;31m======\x1b[m\n");
    printf("Memory access outside mapped regions: %p\n", address);
    printf("Forwarding to native SIGSEGV handler\n");
    ReraiseAsNativeSegfault(sig);
  }

  auto offset = address_int - region->Start();
  auto ucontext = reinterpret_cast<ucontext_t*>(platform);
  auto mcontext = &ucontext->uc_mcontext;

  auto mctx_old = *mcontext;
  auto& pc = mcontext->gregs[REG_RIP];

  MachineCodeInterceptor interceptor(pc, region.get(), offset, mcontext);

  // interceptor.ShowCode();

  pc = interceptor.Intercept();

  auto& mctx_new = *mcontext;

  /* for (size_t i = 0; i < __NGREG; i++) {
    if (mctx_old.gregs[i] != mctx_new.gregs[i]) {
      printf("reg%zu: 0x%llx -> 0x%llx, ", i, mctx_old.gregs[i],
             mctx_new.gregs[i]);
    }
  }
  printf("\n"); */
  // interceptor.ShowCode();
}
}  // namespace

class Emulator {
 public:
  Emulator() {
    struct sigaction sa;

    sa.sa_flags = SA_SIGINFO | SA_NODEFER;
    sigemptyset(&sa.sa_mask);
    sa.sa_sigaction = handler;
    if (sigaction(SIGSEGV, &sa, NULL) == -1) {
      printf("Registering SIGSEGV handler failed\n");
      exit(EXIT_FAILURE);
    }

    mmap(reinterpret_cast<void*>(0xABCD0000), 0x10000, PROT_READ | PROT_WRITE,
         MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    mcu_emulator::hardware::gEmuBridge.Init();

    using mcu_emulator::mmio::ADCRegion;
    using mcu_emulator::mmio::CANRegion;
    using mcu_emulator::mmio::DMARegion;
    using mcu_emulator::mmio::FlashRegion;
    using mcu_emulator::mmio::GPIORegion;
    using mcu_emulator::mmio::NVICRegion;
    using mcu_emulator::mmio::RCCRegion;
    using mcu_emulator::mmio::SCBRegion;
    using mcu_emulator::mmio::TIMRegion;
    using mcu_emulator::mmio::USARTRegion;

    regions.push_back(std::make_shared<SCBRegion>(0xE000ED00));
    regions.push_back(std::make_shared<NVICRegion>(emu_, 0xE000E100));
    regions.push_back(std::make_shared<RCCRegion>(emu_));
    regions.push_back(std::make_shared<GPIORegion<0>>());
    regions.push_back(std::make_shared<GPIORegion<1>>());
    regions.push_back(std::make_shared<GPIORegion<2>>());
    regions.push_back(std::make_shared<GPIORegion<3>>());
    regions.push_back(std::make_shared<GPIORegion<5>>());
    regions.push_back(std::make_shared<USARTRegion<1>>(emu_, 0x40013800));
    regions.push_back(std::make_shared<USARTRegion<2>>(emu_, 0x40004400));
    regions.push_back(std::make_shared<USARTRegion<3>>(emu_, 0x40004800));
    regions.push_back(std::make_shared<CANRegion<0>>(0x40006400));
    regions.push_back(std::make_shared<DMARegion>(0x40020000));
    regions.push_back(std::make_shared<ADCRegion<1>>(0x50000000));
    regions.push_back(std::make_shared<ADCRegion<2>>(0x50000100));
    // regions.push_back(std::make_shared<TIMRegion<1>>(0x40012C00));
    // regions.push_back(std::make_shared<TIMRegion<2>>(0x40000000));
    // regions.push_back(std::make_shared<TIMRegion<3>>(0x40000400));
    regions.push_back(std::make_shared<TIMRegion<6>>(emu_, 0x40001000));
    // regions.push_back(std::make_shared<TIMRegion<7>>(0x40001400));
    // regions.push_back(std::make_shared<TIMRegion<15>>(0x40014000));
    // regions.push_back(std::make_shared<TIMRegion<16>>(0x40014400));
    // regions.push_back(std::make_shared<TIMRegion<17>>(0x40014800));
    regions.push_back(std::make_shared<FlashRegion>(0x40022000));

    for (auto& region : regions) {
      region->reset();
    }

    emu_.network.Connect("emu.sock");
  }

 private:
  mcu_emulator::hardware::Emu emu_;
};

extern "C" void InitRCC();
extern "C" void InitVector();

__attribute__((constructor)) void InitEmulator() {
  static Emulator emulator;

  InitRCC();
  InitVector();
}

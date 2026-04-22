#include <cerrno>
#include <csignal>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstdlib>

#include <sys/mman.h>
#include <sys/ptrace.h>
#include <sys/signal.h>
#include <ucontext.h>
#include <unistd.h>

#include <memory>
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
#include "mmio/nvic.hpp"
#include "mmio/rcc.hpp"
#include "mmio/scb.hpp"
#include "mmio/timer.hpp"
#include "mmio/usart.hpp"

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

static void handler(int sig, siginfo_t* si, void* platform) {
  constexpr int kRegEFL = 17;

  auto address = si->si_addr;
  auto address_int = reinterpret_cast<uintptr_t>(address);

  auto region = LookupRegion(address_int);
  if (!region) {
    printf(
        "\x1b[1;31m======\x1b[m Segmentation Fault. "
        "\x1b[1;31m======\x1b[m\n");
    printf("Memory access outside mapped regions: %p\n", address);
    printf("Signal handler finished\n");
    while (1)
      ;
  }

  auto offset = address_int - region->Start();
  auto ucontext = reinterpret_cast<ucontext_t*>(platform);
  auto mcontext = &ucontext->uc_mcontext;

  auto& pc = mcontext->gregs[REG_RIP];

  // printf("\x1b[1;32m--------\x1b[m MMIO Trap \x1b[1;32m--------\x1b[m\n");

  using mcu_emulator::machine_code::ModRM;
  using mcu_emulator::machine_code::SIB;

  auto code_buf = reinterpret_cast<uint8_t*>(pc);
  if (code_buf[0] == 0x8B) {  // MOV Ev, Gv
    ModRM modrm(code_buf[1]);
    if (modrm.GetMod() == ModRM::Mod::MOD_NO_DISP) {
      auto value = region->read(offset);
      auto reg = modrm.GetReg();
      // auto rm = modrm.GetRm();
      const auto inst_len = 2;
      // printf("%s <-- [%s] (==> 0x%08x)\n", reg.ToString().c_str(),
      //        rm.ToString().c_str(), value);

      reg.Write(mcontext, value);

      pc += inst_len;  // Skip instruction
      // printf("SIGSEGV: %p --> %08x\n", address, value);
      // printf("SIGSEGV: PC: %llx (skiped %d Bytes)\n", pc, inst_len);
      // printf("\x1b[2;32m--------\x1b[m MMIO Trap \x1b[2;32m--------\x1b[m\n");
      return;
    } else if (modrm.GetMod() == ModRM::Mod::MOD_DISP8) {
      auto value = region->read(offset);
      // auto disp = code_buf[2];
      auto reg = modrm.GetReg();
      // auto rm = modrm.GetRm();
      const auto inst_len = 3;
      // printf("%s <-- [%s + 0x%04x] (==> 0x%08x)\n", reg.ToString().c_str(),
      //        rm.ToString().c_str(), disp, value);

      reg.Write(mcontext, value);

      pc += inst_len;  // Skip instruction
      // printf("SIGSEGV: %p --> %08x\n", address, value);
      // printf("SIGSEGV: PC: %llx (skiped %d Bytes)\n", pc, inst_len);
      // printf("\x1b[2;32m--------\x1b[m MMIO Trap \x1b[2;32m--------\x1b[m\n");
      return;
    } else if (modrm.GetMod() == ModRM::Mod::MOD_DISP32) {
      auto value = region->read(offset);
      // auto disp = *reinterpret_cast<uint32_t*>(code_buf + 2);
      auto reg = modrm.GetReg();
      // auto rm = modrm.GetRm();
      const auto inst_len = 6;
      // printf("%s <-- [%s + 0x%04x] (==> 0x%08x)\n", reg.ToString().c_str(),
      //        rm.ToString().c_str(), disp, value);

      reg.Write(mcontext, value);

      pc += inst_len;  // Skip instruction
      // printf("SIGSEGV: %p --> %08x\n", address, value);
      // printf("SIGSEGV: PC: %llx (skiped %d Bytes)\n", pc, inst_len);
      // printf("\x1b[2;32m--------\x1b[m MMIO Trap \x1b[2;32m--------\x1b[m\n");
      return;
    }
  } else if (code_buf[0] == 0x66) {  // 16 Bit prefix
    if (code_buf[1] == 0x89) {
      ModRM modrm(code_buf[2]);
      if (modrm.GetMod() == ModRM::Mod::MOD_DISP8) {
        // auto disp = code_buf[3];
        auto reg = modrm.GetReg();
        // auto rm = modrm.GetRm();
        const auto inst_len = 4;
        auto value = static_cast<uint32_t>(reg.Read(mcontext)) & 0xFFFF;

        // printf("[%s + 0x%04x] <-- %s (==> 0x%04x) \n", rm.ToString().c_str(),
        //        disp, reg.ToString().c_str(), value);
        region->write_u32(offset, value);

        pc += inst_len;  // Skip instruction
        // printf("SIGSEGV: %p <-- %08x\n", address, value);
        // printf("SIGSEGV: PC: %llx (skiped %d Bytes)\n", pc, inst_len);
        // printf("\x1b[2;32m--------\x1b[m MMIO Trap \x1b[2;32m--------\x1b[m\n");
        return;
      }
    }
  } else if (code_buf[0] == 0x89) {  // Mov Gv, Ev
    ModRM modrm(code_buf[1]);
    if (modrm.GetMod() == ModRM::Mod::MOD_NO_DISP) {
      auto reg = modrm.GetReg();
      auto rm = modrm.GetRm();
      const auto inst_len = 2;
      auto value = static_cast<uint32_t>(reg.Read(mcontext));

      // printf("[%s] <-- %s (==> 0x%08x) \n", rm.ToString().c_str(),
      //        reg.ToString().c_str(), value);
      region->write_u32(offset, value);

      pc += inst_len;  // Skip instruction
      // printf("SIGSEGV: %p <-- %08x\n", address, value);
      // printf("SIGSEGV: PC: %llx (skiped %d Bytes)\n", pc, inst_len);
      // printf("\x1b[2;32m--------\x1b[m MMIO Trap \x1b[2;32m--------\x1b[m\n");
      return;
    } else if (modrm.GetMod() == ModRM::Mod::MOD_DISP8) {
      auto disp = code_buf[2];
      auto reg = modrm.GetReg();
      auto rm = modrm.GetRm();
      const auto inst_len = 3;
      auto value = static_cast<uint32_t>(reg.Read(mcontext));

      // printf("[%s + 0x%04x] <-- %s (==> 0x%08x) \n", rm.ToString().c_str(),
      //        disp, reg.ToString().c_str(), value);
      region->write_u32(offset, value);

      pc += inst_len;  // Skip instruction
      // printf("SIGSEGV: %p <-- %08x\n", address, value);
      // printf("SIGSEGV: PC: %llx (skiped %d Bytes)\n", pc, inst_len);
      // printf("\x1b[2;32m--------\x1b[m MMIO Trap \x1b[2;32m--------\x1b[m\n");
      return;
    } else if (modrm.GetMod() == ModRM::Mod::MOD_DISP32) {
      auto disp = *reinterpret_cast<uint32_t*>(code_buf + 2);
      auto reg = modrm.GetReg();
      auto rm = modrm.GetRm();
      const auto inst_len = 6;
      auto value = static_cast<uint32_t>(reg.Read(mcontext));

      // printf("[%s + 0x%04x] <-- %s (==> 0x%08x) \n", rm.ToString().c_str(),
      //        disp, reg.ToString().c_str(), value);
      region->write_u32(offset, value);

      pc += inst_len;  // Skip instruction
      // printf("SIGSEGV: %p <-- %08x\n", address, value);
      // printf("SIGSEGV: PC: %llx (skiped %d Bytes)\n", pc, inst_len);
      // printf("\x1b[2;32m--------\x1b[m MMIO Trap \x1b[2;32m--------\x1b[m\n");
      return;
    }
  } else if (code_buf[0] == 0xc7) {  // Mov Gv, Ev
    ModRM modrm(code_buf[1]);
    if (modrm.GetMod() == ModRM::Mod::MOD_DISP8) {
      auto disp = code_buf[2];
      auto imm = *reinterpret_cast<uint32_t*>(code_buf + 3);
      auto rm = modrm.GetRm();
      const auto inst_len = 7;

      // printf("[%s + 0x%02x] <-- 0x%08x \n", rm.ToString().c_str(), disp, imm);

      region->write_u32(offset, imm);

      pc += inst_len;  // Skip instruction
      // printf("SIGSEGV: %p <-- %08x\n", address, value);
      // printf("SIGSEGV: PC: %llx (skiped %d Bytes)\n", pc, inst_len);
      // printf("\x1b[2;32m--------\x1b[m MMIO Trap \x1b[2;32m--------\x1b[m\n");
      return;
    } else if (modrm.GetMod() == ModRM::Mod::MOD_NO_DISP) {
      auto imm = *reinterpret_cast<uint32_t*>(code_buf + 2);
      auto rm = modrm.GetRm();
      const auto inst_len = 6;

      // printf("[%s] <-- 0x%08x \n", rm.ToString().c_str(), imm);

      region->write_u32(offset, imm);

      pc += inst_len;  // Skip instruction
      // printf("SIGSEGV: %p <-- %08x\n", address, value);
      // printf("SIGSEGV: PC: %llx (skiped %d Bytes)\n", pc, inst_len);
      // printf("\x1b[2;32m--------\x1b[m MMIO Trap \x1b[2;32m--------\x1b[m\n");
      return;
    }
  } else if (code_buf[0] == 0x88) {  // mov rm <-- reg (8bit)
    ModRM modrm(code_buf[1]);
    if (modrm.GetMod() == ModRM::Mod::MOD_DISP32) {
      if (modrm.GetRm().Index() == REG_RSP) {
        SIB sib(code_buf[2]);
        auto disp = *reinterpret_cast<uint32_t*>(code_buf + 3);
        const auto inst_len = 7;

        auto value = modrm.GetReg().Read(mcontext) & 0xFF;
        region->write_u8(offset, value);

        pc += inst_len;  // Skip instruction
      }
      return;
    }
  } else if (code_buf[0] == 0x0f and code_buf[1] == 0xb7) {  // reg32 <-- rm8
    auto modrm = ModRM(code_buf[2]);
    if (modrm.GetMod() == ModRM::Mod::MOD_DISP8) {
      const auto inst_len = 4;
      auto reg = modrm.GetReg();

      reg.Write(mcontext, region->read(offset));

      pc += inst_len;  // Skip instruction
      return;
    }
  } else if (code_buf[0] == 0x83) {
    ModRM modrm(code_buf[1]);
    if (modrm.GetMod() == ModRM::Mod::MOD_NO_DISP) {
      auto mode = modrm.GetReg();
      auto rm = modrm.GetRm();
      void* mem_addr = nullptr;
      uint8_t imm = 0;
      int code_len = 0;
      if (rm.Encode() == 0b101) {
        printf("Unhandled instruction: 83h with rm=0b101[rbp] (disp32)\n");
      }
      if (rm.Encode() == 0b100) {
        SIB sib(code_buf[2]);
        auto index =
            sib.Index().Encode() == 0b100 ? 0 : sib.Index().Read(mcontext);
        auto scale = 1 << static_cast<int>(sib.Scale());
        uint32_t base = 0;
        if (sib.Base().Encode() == 0b101) {
          base |= code_buf[3];
          base |= static_cast<uint32_t>(code_buf[4]) << 8;
          base |= static_cast<uint32_t>(code_buf[5]) << 16;
          base |= static_cast<uint32_t>(code_buf[6]) << 24;
          code_len = 7;
        } else {
          base = sib.Base().Read(mcontext);
          code_len = 3;
        }
        mem_addr = reinterpret_cast<void*>(base + index * scale);
      } else {
        mem_addr = reinterpret_cast<void*>(rm.Read(mcontext));
        code_len = 2;
      }
      imm = code_buf[code_len];
      const auto inst_len = code_len + 1;

      if (mode.Encode() == 0b001) {  // OR
        uint32_t value = region->read(offset);
        value |= imm;
        region->write_u32(offset, value);

        pc += inst_len;  // Skip instruction
      } else {
        printf("Unhandled instruction: 83h with mod=0b00 and reg=0b%03b\n",
               mode.Encode());
      }

      return;
    }
  } else if (code_buf[0] == 0x81) {
    ModRM modrm(code_buf[1]);
    if (modrm.GetMod() == ModRM::Mod::MOD_NO_DISP) {
      auto mode = modrm.GetReg();
      auto rm = modrm.GetRm();
      void* mem_addr = nullptr;
      uint32_t imm = 0;
      int code_len = 2;
      if (rm.Encode() == 0b101) {
        printf("Unhandled instruction: 81h with rm=0b101[rbp] (disp32)\n");
      }
      if (rm.Encode() == 0b100) {
        SIB sib(code_buf[2]);
        code_len += 1;
        auto index =
            sib.Index().Encode() == 0b100 ? 0 : sib.Index().Read(mcontext);
        auto scale = 1 << static_cast<int>(sib.Scale());
        uint32_t base = 0;
        if (sib.Base().Encode() == 0b101) {
          base |= code_buf[3];
          base |= static_cast<uint32_t>(code_buf[4]) << 8;
          base |= static_cast<uint32_t>(code_buf[5]) << 16;
          base |= static_cast<uint32_t>(code_buf[6]) << 24;
          code_len += 4;
        } else {
          base = sib.Base().Read(mcontext);
        }
        mem_addr = reinterpret_cast<void*>(base + index * scale);
      } else {
        mem_addr = reinterpret_cast<void*>(rm.Read(mcontext));
      }
      imm = *reinterpret_cast<uint32_t*>(code_buf + code_len);
      const auto inst_len = code_len + 4;

      if (mode.Encode() == 0b001) {
        uint32_t value = region->read(offset);
        value |= imm;
        region->write_u32(offset, value);

        pc += inst_len;  // Skip instruction
        return;
      } else if (mode.Encode() == 0b100) {  // AND
        uint32_t value = region->read(offset);
        value &= imm;
        region->write_u32(offset, value);

        pc += inst_len;  // Skip instruction
        return;
      }
      printf("Unhandled instruction: 81h with mod=0b00 and reg=0b%03b\n",
             mode.Encode());
    }
  } else if (code_buf[0] == 0x0F and code_buf[1] == 0xBA) {
    size_t inst_len = 2;

    ModRM modrm(code_buf[inst_len]);
    auto mode = modrm.GetReg();
    auto rm = modrm.GetRm();
    inst_len++;

    void* mem_addr = nullptr;

    if (modrm.GetMod() == ModRM::Mod::MOD_NO_DISP) {
      SIB sib(code_buf[inst_len]);
      inst_len++;

      auto index =
          sib.Index().Encode() == 0b100 ? 0 : sib.Index().Read(mcontext);
      auto scale = 1 << static_cast<int>(sib.Scale());
      uint32_t base = 0;
      if (sib.Base().Encode() == 0b101) {
        base |= code_buf[inst_len];
        base |= static_cast<uint32_t>(code_buf[inst_len + 1]) << 8;
        base |= static_cast<uint32_t>(code_buf[inst_len + 2]) << 16;
        base |= static_cast<uint32_t>(code_buf[inst_len + 3]) << 24;
        inst_len += 4;
      } else {
        base = sib.Base().Read(mcontext);
      }
      mem_addr = reinterpret_cast<void*>(base + index * scale);

      uint8_t imm = code_buf[inst_len];
      inst_len++;

      if (rm.Encode() == 0b100) {
        // BT
        uint32_t value = region->read(offset);
        bool bit = (value >> imm) & 1;
        mcontext->gregs[kRegEFL] =
            (mcontext->gregs[kRegEFL] & ~0x40) | (bit << 6);
        pc += inst_len;  // Skip instruction
        return;
      } else {
        printf("Unhandled instruction: 0FBAh with rm=0b%03b (mod=0b00)\n",
               rm.Encode());
      }
    }
  }

  printf(
      "\x1b[1;31m======\x1b[m Unhandled instruction "
      "\x1b[1;31m======\x1b[m\n");
  printf("Attempting to access %p failed\n", address);
  printf("code: %016llx\n", pc);
  for (int j = 0; j <= 15; j++) {
    for (int i = 0; i < 16; i++) {
      auto value = code_buf[j * 16 + i];
      printf("%02x ", value);
    }
    printf("\n");
  }

  printf("Signal handler finished\n");
  while (1)
    ;
}

static void abort_handler(int sig, siginfo_t* si, void* platform) {
  while (true) {
    // sleep 2s
    sleep(2);
  }
}

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

    sa.sa_flags = SA_SIGINFO | SA_NODEFER;
    sigemptyset(&sa.sa_mask);
    sa.sa_sigaction = abort_handler;
    if (sigaction(SIGABRT, &sa, NULL) == -1) {
      printf("Registering SIGSEGV handler failed\n");
      exit(EXIT_FAILURE);
    }

    mmap(reinterpret_cast<void*>(0xABCD0000), 0x10000, PROT_READ | PROT_WRITE,
         MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    mcu_emulator::hardware::gEmuBridge.Init();

    static mcu_emulator::hardware::Emu emu;
    emu.network.MakeTestPacketDump();
    emu.network.Connect("emu.sock");

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
    regions.push_back(std::make_shared<NVICRegion>(emu, 0xE000E100));
    regions.push_back(std::make_shared<RCCRegion>(emu));
    regions.push_back(std::make_shared<GPIORegion<0>>());
    regions.push_back(std::make_shared<GPIORegion<1>>());
    regions.push_back(std::make_shared<GPIORegion<2>>());
    regions.push_back(std::make_shared<GPIORegion<3>>());
    regions.push_back(std::make_shared<GPIORegion<5>>());
    regions.push_back(std::make_shared<USARTRegion<1>>(emu, 0x40013800));
    regions.push_back(std::make_shared<USARTRegion<2>>(emu, 0x40004400));
    regions.push_back(std::make_shared<USARTRegion<3>>(emu, 0x40004800));
    regions.push_back(std::make_shared<CANRegion<0>>(0x40006400));
    regions.push_back(std::make_shared<DMARegion>(0x40020000));
    regions.push_back(std::make_shared<ADCRegion<1>>(0x50000000));
    regions.push_back(std::make_shared<ADCRegion<2>>(0x50000100));
    // regions.push_back(std::make_shared<TIMRegion<1>>(0x40012C00));
    // regions.push_back(std::make_shared<TIMRegion<2>>(0x40000000));
    // regions.push_back(std::make_shared<TIMRegion<3>>(0x40000400));
    regions.push_back(std::make_shared<TIMRegion<6>>(emu, 0x40001000));
    // regions.push_back(std::make_shared<TIMRegion<7>>(0x40001400));
    // regions.push_back(std::make_shared<TIMRegion<15>>(0x40014000));
    // regions.push_back(std::make_shared<TIMRegion<16>>(0x40014400));
    // regions.push_back(std::make_shared<TIMRegion<17>>(0x40014800));
    regions.push_back(std::make_shared<FlashRegion>(0x40022000));

    for (auto& region : regions) {
      region->reset();
    }
  }
};

extern "C" void InitRCC();
extern "C" void InitVector();

__attribute__((constructor)) void InitEmulator() {
  static Emulator emulator;

  InitRCC();
  InitVector();
}

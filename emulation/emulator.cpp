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
#include <vector>
#include "hardware/bridge.hpp"
#include "hardware/mcu.hpp"
#include "machine_code/interceptor.hpp"
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

  mcu_emulator::machine_code::MachineCodeInterceptor interceptor(
      pc, region.get(), offset, mcontext);

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

#include "f3xx-mmio-simulator.h"

__attribute__((constructor)) void InitEmulator() {
  // static Emulator emulator;
  init_mmio_simulator();

  InitRCC();
  InitVector();
}

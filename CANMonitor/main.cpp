#include "can.hpp"
#include "can_debug.hpp"
#include "can_debug_seq.hpp"
#include "event_log.hpp"
#include "rcc.hpp"

#include <f3/console.hpp>

using App = CanDebug;
// using App = CANMonitor::CANDebug_Seq;

struct HardwareConfig {
  using RCCConfig = CANMonitor::BaremetalRCC;

  using ConsoleTx = stm32f3::GPIO<0, 2>;
  using ConsoleRx = stm32f3::GPIO<0, 15>;
  static constexpr uint32_t kConsoleBaudrate = 921600;
  static constexpr uint32_t kConsoleUARTAltFn = 7;
  static constexpr uint32_t kConsoleUARTId = 2;
  static constexpr size_t kConsoleRxBufSize = 0;
};

int main() {
  stm32::InitRCC();
  stm32f3::Console<HardwareConfig>::Init();
  CANMonitor::InitCAN();

  CANMonitor::kEventLog.Log("Is RCC Initialized?: %d",
                            CANMonitor::rcc_initialized);

  App app;
  app.Main();
}
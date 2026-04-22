#pragma once

#include <functional>

#include <f3/peripherals/can.hpp>
#include <f3/peripherals/gpio.hpp>
#include "rcc.hpp"

namespace CANMonitor {
//* Configuration
class Handler {
 public:
  static void HandleRx(int fifo, stm32f3::can::CANMessage const& msg) {
    if (handle_rx_)
      handle_rx_(fifo, msg);
  }
  static void HandleError() {
    if (handle_error_)
      handle_error_();
  }

  static void Init(
      std::function<void(int fifo, stm32f3::can::CANMessage const& msg)>
          handle_rx,
      std::function<void()> handle_error) {
    handle_rx_ = handle_rx;
    handle_error_ = handle_error;
  }

 private:
  static inline std::function<void(int fifo,
                                   stm32f3::can::CANMessage const& msg)>
      handle_rx_ = nullptr;
  static inline std::function<void()> handle_error_ = nullptr;
};

using AppCAN = stm32f3::can::BaremetalCAN<Handler>;
inline void InitCAN() {
  stm32f3::GPIO<0, 11>::InitAsAF<9>();  // PA_11: CAN_RX (AF9)
  stm32f3::GPIO<0, 12>::InitAsAF<9>();  // PA_12: CAN_TX (AF9)
  AppCAN::Init<CANMonitor::BaremetalRCC, (int)1e6>();
}

}  // namespace CANMonitor

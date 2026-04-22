#pragma once

#include <optional>
#include "mmio.hpp"

#include "../hardware/mcu.hpp"

namespace mcu_emulator::mmio {
template <int index>
class USARTRegion : public MMIORegion {
  auto GetEPMarker() const {
    return EndpointMarker{
        .kind = mcu_emulator::EndpointMarker::Kind::kUART,
        .id = index,
    };
  }

 public:
  USARTRegion(hardware::Emu& emu, uint32_t base)
      : MMIORegion(base, 0x400), emu_(emu) {
    emu_.network.RegisterDataCallback(
        GetEPMarker(), [this](const DataMessage& msg) {
          for (auto byte : msg.data) {
            this->rx_char = byte;
            emu_.nvic.FireInterrupt(hardware::IRQn_Type::USART1_IRQn);
            this->rx_char = std::nullopt;
          }
        });
  }

  uint32_t read(uint32_t offset) final {
    if (offset == 0) {
      auto ret = 0;
      ret |= tx_enabled ? (0x1UL << (3U)) : 0;   // TX Enabled
      ret |= rx_enabled ? (0x1UL << (2U)) : 0;   // RX Enabled
      ret |= drv_enabled ? (0x1UL << (0U)) : 0;  // Driver Enabled
      return ret;
    }
    if (offset == 0x24) {  // RDR
      return rx_char.value_or(0);
    }

    if (offset == 0x1C) {  // ISR
      auto ret = 0;
      ret |= rx_char.has_value() ? (0x1UL << (5U)) : 0;  // RX Not Empty
      ret |= (0x1UL << (6U));                            // TX Completed
      ret |= (0x1UL << (7U));                            // TX Empty

      return ret;
    }

    printf("Read from USART: %08x\n", offset);
    throw std::runtime_error("Invalid read from USART");
  }

  void write_u32(uint32_t offset, uint32_t value) final {
    if (offset == 0) {
      tx_enabled = value & (0x1UL << (3U)) ? true : false;   // TX Enabled
      rx_enabled = value & (0x1UL << (2U)) ? true : false;   // RX Enabled
      drv_enabled = value & (0x1UL << (0U)) ? true : false;  // Driver Enabled
      return;
    }
    if (offset == 0x28) {  // TDR
      emu_.network.SendMessage(GetEPMarker(), {static_cast<uint8_t>(value)});
      return;
    }
    if (offset == 0x0C) {  // BRR
      return;
    }

    printf("Write to USART+%08x: %08x\n", offset, value);
    throw std::runtime_error("Invalid write to USART");
  }

  void reset() override {}

 private:
  std::optional<uint8_t> rx_char;
  bool tx_enabled = false;
  bool rx_enabled = false;
  bool drv_enabled = false;
  hardware::Emu& emu_;
};
}  // namespace mcu_emulator::mmio

#pragma once

#include <cstdint>
#include <cstdio>

#include <array>

#include <Nano/fixed_map.hpp>

#include "can.hpp"
#include "f3/peripherals/can.hpp"
#include "tick_timer.hpp"

class CanDebug {
  const int kHeaderLines = 4;

  struct CanMessageData {
    static inline uint8_t last_line;

    bool is_invalidated = true;  ///! Status of the data
    uint8_t line = last_line;    ///! [Visual] Line number on the screen
    std::array<std::uint8_t, 8> data = {};  ///! [Data] Content
    uint8_t length = 0;                     ///! [Data] Length of the data
    int rx_count = 0;                       ///! [Stat] Receive count

    static void AddLine() { last_line++; }
  };

  void InitScreen() const {
    printf("\x1b[2J\x1b[1;1H");
    printf("\x1b[?25l");
  }

  void UpdateScreen() {
    for (auto&& pair : messages_) {
      auto id = pair.key;
      auto& data = pair.value;

      if (!data->is_invalidated) {
        continue;
      }

      printf("\x1b[%d;1H", kHeaderLines + 1 + data->line);
      printf("%8d] %08X (%5u):", tick_, id, data->rx_count);
      for (size_t i = 0; i < data->length; i++) {
        printf(" %02X", data->data[i]);
      }
      printf("\x1b[0K\n");

      data->is_invalidated = false;
    }
  }

  void ShowHeader() {
    printf("\x1b[1;1H");
    printf("\x1b[2K");
    printf("Tick: %5d\x1b[0K\n", tick_);
    printf("Messages: %5u\x1b[0K\n", messages_count_);
    printf("Last Failed: %5d\x1b[0K\n", last_failed_tick_);
  }

  void Init() {
    InitScreen();

    printf("CAN Monitor App (%s)\n", __TIME__);
    printf("Initializing...\n");
    fflush(stdout);

    InitScreen();
  }

 public:
  CanDebug() {
    auto rx = ([this](int fifo, stm32f3::can::CANMessage const& msg) {
      auto const& id = msg.id;
      auto const& data = msg.data;

      if (!messages_.Contains(id)) {
        messages_[id] = CanMessageData();
        messages_count_++;
        CanMessageData::AddLine();
      }

      auto&& msg_stat = messages_[id];
      msg_stat.is_invalidated = true;
      msg_stat.rx_count++;
      msg_stat.length = static_cast<uint8_t>(data.size());
      msg_stat.data = data;
    });
    auto err = [] {
    };

    CANMonitor::Handler::Init(rx, err);
  }

  [[noreturn]] void Main() {
    Init();

    while (true) {
      UpdateScreen();
      ShowHeader();
      tick_++;

      WaitMS(10);
    }
  }

 private:
  Nano::collection::FixedMap<uint32_t, CanMessageData, 32> messages_;
  uint32_t messages_count_ = 0;
  int tick_ = 0;
  int last_failed_tick_ = 0;
};
#include <f3/eventlog.hpp>
#include "f3/peripherals/can.hpp"

#include "can.hpp"
#include "event_log.hpp"
#include "tick_timer.hpp"
#include "utils.hpp"

namespace CANMonitor {
class CANDebug_Seq {

 public:
  CANDebug_Seq() {}

  void Main() {
    using CANMonitor::AppCAN;
    using LED = stm32f3::GPIO<1, 3>;

    struct Handler {
      static void OnTick() { kEventLog.tick++; }
    };
    InitTimer<Handler>();

    printf("\x1b[2J");  // Clear Screen
    kEventLog.Log("CAN Initialized");

    auto rx = [](int _, stm32f3::can::CANMessage const& msg) {
      auto str = FormatHEX(msg.data.data(), msg.data.size());
      kEventLog.Log("CAN Rx: %08X [%s]", msg.id, str);
    };
    auto err = [] {
    };
    CANMonitor::Handler::Init(rx, err);

    LED::InitAsGPIO();

    int i = 0;
    while (true) {
      //* UI
      printf("\x1b[0;1H");

      printf("F303K8 baremetal CAN Test (loop=%d)" NEWLINE, i);

      auto error_statistic = AppCAN::GetErrorStatistic();
      printf("CAN Status [%s]" NEWLINE, error_statistic.StatusToString());
      printf("  - REC: %d, TEC: %d" NEWLINE, error_statistic.rec,
             error_statistic.tec);
      printf("  - LEC: %s" NEWLINE, error_statistic.LastErrorCodeToString());

      printf("CAN Tx Status" NEWLINE);
      auto mailbox0 = AppCAN::GetTxMailbox<0>();
      printf("  - MailBox0 [%s]: %s" NEWLINE, mailbox0.StatusToString(),
             FormatHEX(mailbox0.GetData().data(), mailbox0.GetDLC()));
      auto mailbox1 = AppCAN::GetTxMailbox<1>();
      printf("  - MailBox1 [%s]: %s" NEWLINE, mailbox1.StatusToString(),
             FormatHEX(mailbox1.GetData().data(), mailbox1.GetDLC()));
      auto mailbox2 = AppCAN::GetTxMailbox<2>();
      printf("  - MailBox2 [%s]: %s" NEWLINE, mailbox2.StatusToString(),
             FormatHEX(mailbox2.GetData().data(), mailbox2.GetDLC()));

      printf("Events\n");
      for (auto log : kEventLog) {
        printf("  - %d: %s" NEWLINE, log->timestamp, log->message);
      }

      //* Blink PB_3
      LED::ToggleGPIO();

      if (i % 100) {
        AppCAN::Send(
            {.id = 0x555, .length = 5, .data = {0x55, 0x55, 0x55, 0x55, 0x55}});
      }

      i++;
      WaitMS(10);
    }
  }
};
}  // namespace CANMonitor
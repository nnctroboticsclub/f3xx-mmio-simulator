#pragma once

#include <cstdio>
#include <stdexcept>
#include "bridge.hpp"

namespace mcu_emulator::hardware {
enum class IRQn_Type {
  /******  Cortex-M4 Processor Exceptions Numbers ****************************************************************/
  NonMaskableInt_IRQn = -14,    // 2 Non Maskable Interrupt
  HardFault_IRQn = -13,         // 3 Cortex-M4 Hard Fault Interrupt
  MemoryManagement_IRQn = -12,  // 4 Cortex-M4 Memory Management Interrupt
  BusFault_IRQn = -11,          // 5 Cortex-M4 Bus Fault Interrupt
  UsageFault_IRQn = -10,        // 6 Cortex-M4 Usage Fault Interrupt
  SVCall_IRQn = -5,             // 11 Cortex-M4 SV Call Interrupt
  DebugMonitor_IRQn = -4,       // 12 Cortex-M4 Debug Monitor Interrupt
  PendSV_IRQn = -2,             // 14 Cortex-M4 Pend SV Interrupt
  SysTick_IRQn = -1,            // 15 Cortex-M4 System Tick Interrupt
  /******  STM32 specific Interrupt Numbers **********************************************************************/
  WWDG_IRQn = 0,             // Window WatchDog Interrupt
  PVD_IRQn = 1,              // PVD through EXTI Line detection Interrupt
  TAMP_STAMP_IRQn = 2,       // Tamper and TimeStamp interrupts
                             //   through the EXTI line 19
  RTC_WKUP_IRQn = 3,         // RTC Wakeup interrupt through the EXTI line 20
  FLASH_IRQn = 4,            // FLASH global Interrupt
  RCC_IRQn = 5,              // RCC global Interrupt
  EXTI0_IRQn = 6,            // EXTI Line0 Interrupt
  EXTI1_IRQn = 7,            // EXTI Line1 Interrupt
  EXTI2_TSC_IRQn = 8,        // EXTI Line2 Interrupt
                             //   Touch Sense Controller Interrupt
  EXTI3_IRQn = 9,            // EXTI Line3 Interrupt
  EXTI4_IRQn = 10,           // EXTI Line4 Interrupt
  DMA1_Channel1_IRQn = 11,   // DMA1 Channel 1 Interrupt
  DMA1_Channel2_IRQn = 12,   // DMA1 Channel 2 Interrupt
  DMA1_Channel3_IRQn = 13,   // DMA1 Channel 3 Interrupt
  DMA1_Channel4_IRQn = 14,   // DMA1 Channel 4 Interrupt
  DMA1_Channel5_IRQn = 15,   // DMA1 Channel 5 Interrupt
  DMA1_Channel6_IRQn = 16,   // DMA1 Channel 6 Interrupt
  DMA1_Channel7_IRQn = 17,   // DMA1 Channel 7 Interrupt
  ADC1_2_IRQn = 18,          // ADC1 & ADC2 Interrupts
  CAN_TX_IRQn = 19,          // CAN TX Interrupt
  CAN_RX0_IRQn = 20,         // CAN RX0 Interrupt
  CAN_RX1_IRQn = 21,         // CAN RX1 Interrupt
  CAN_SCE_IRQn = 22,         // CAN SCE Interrupt
  EXTI9_5_IRQn = 23,         // External Line[9:5] Interrupts
  TIM1_BRK_TIM15_IRQn = 24,  // TIM1 Break and TIM15 Interrupts
  TIM1_UP_TIM16_IRQn = 25,   // TIM1 Update and TIM16 Interrupts
  TIM1_TRG_COM_TIM17_IRQn = 26,  // TIM1 Trigger and Commutation and
                                 //   TIM17 Interrupt
  TIM1_CC_IRQn = 27,             // TIM1 Capture Compare Interrupt
  TIM2_IRQn = 28,                // TIM2 global Interrupt
  TIM3_IRQn = 29,                // TIM3 global Interrupt
  I2C1_EV_IRQn = 31,             // I2C1 Event Interrupt
                                 //   & EXTI Line23 Interrupt (I2C1 wakeup)
  I2C1_ER_IRQn = 32,             // I2C1 Error Interrupt
  SPI1_IRQn = 35,                // SPI1 global Interrupt
  USART1_IRQn = 37,              // USART1 global Interrupt
                                 //  & EXTI Line25 Interrupt (USART1 wakeup)
  USART2_IRQn = 38,              // USART2 global Interrupt
                                 //  & EXTI Line26 Interrupt (USART2 wakeup)
  USART3_IRQn = 39,              // USART3 global Interrupt
                                 //  & EXTI Line28 Interrupt (USART3 wakeup)
  EXTI15_10_IRQn = 40,           // External Line[15:10] Interrupts
  RTC_Alarm_IRQn = 41,  // RTC Alarm (A and B) through EXTI Line 17 Interrupt
  TIM6_DAC1_IRQn = 54,  // TIM6 global and DAC1 underrun error Interrupts
  TIM7_DAC2_IRQn = 55,  // TIM7 global
                        // and DAC2 channel1 underrun error Interrupt
  COMP2_IRQn = 64,      // COMP2 global Interrupt via EXTI Line22
  COMP4_6_IRQn = 65,  // COMP4 and COMP6 global Interrupt via EXTI Line30 and 32
  FPU_IRQn = 81,      // Floating point Interrupt
};

class IRQn {
 public:
  IRQn(int irq_index) : type_(static_cast<IRQn_Type>(irq_index)) {}
  IRQn(IRQn_Type type) : type_(type) {}
  operator IRQn_Type() const { return type_; }
  operator int() const { return static_cast<int>(type_); }

  bool operator==(const IRQn& other) const { return type_ == other.type_; }
  bool operator!=(const IRQn& other) const { return type_ != other.type_; }

  auto ToVecTabIndex() const { return static_cast<int>(type_) + 16; }

 private:
  IRQn_Type type_;
};

class NVIC {
  class IRQStatus {
   public:
    auto SetEnabled(bool enabled) {
      if (not(enabled_ xor enabled)) {
        return;
      }
      enabled_ = enabled;
    }

    auto IsEnabled() { return enabled_; }

   private:
    bool enabled_;
  };

 public:
  auto SetEnableInterrupt(IRQn irq, bool enabled) {
    irqs[irq.ToVecTabIndex()].SetEnabled(enabled);
  }

  auto IsEnabledInterrupt(IRQn irq) {
    return irqs[irq.ToVecTabIndex()].IsEnabled();
  }

  auto FireInterrupt(IRQn irq) {
    if (not irqs[irq.ToVecTabIndex()].IsEnabled()) {
      return;
    }

    auto handler = gEmuBridge.vector_table->entries[irq.ToVecTabIndex()];
    if (handler) {
      handler();
    } else {
      printf("No handler for IRQ %d\n", static_cast<int>(irq));
      throw std::runtime_error("No handler for IRQ");
    }
  }

 public:
  IRQStatus irqs[0x200 / 4];
};
}  // namespace mcu_emulator::hardware

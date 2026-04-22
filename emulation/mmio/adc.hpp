#pragma once

#include <cstdint>
#include <cstdio>
#include "mmio.hpp"
namespace mcu_emulator::mmio {
// TODO: Implement ADC
template <int index>
class ADCRegion : public MMIORegion {
  struct ADC_TypeDef {
    uint32_t ISR = 0;          // ADC Interrupt and Status Register,
    uint32_t IER = 0;          // ADC Interrupt Enable Register,
    uint32_t CR = 0x40000000;  // ADC control register,
    uint32_t CFGR = 0;         // ADC Configuration register,
    uint32_t RESERVED0 = 0;
    uint32_t SMPR1 = 0;  // ADC sample time register 1,
    uint32_t SMPR2 = 0;  // ADC sample time register 2,
    uint32_t RESERVED1 = 0;
    uint32_t TR1 = 0x0fff0000;  // ADC watchdog threshold register 1,
    uint32_t TR2 = 0x00ff0000;  // ADC watchdog threshold register 2,
    uint32_t TR3 = 0x00ff0000;  // ADC watchdog threshold register 3,
    uint32_t RESERVED2 = 0;
    uint32_t SQR1 = 0;  // ADC regular sequence register 1,
    uint32_t SQR2 = 0;  // ADC regular sequence register 2,
    uint32_t SQR3 = 0;  // ADC regular sequence register 3,
    uint32_t SQR4 = 0;  // ADC regular sequence register 4,
    uint32_t DR = 0;    // ADC regular data register,
    uint32_t RESERVED3 = 0;
    uint32_t RESERVED4 = 0;
    uint32_t JSQR = 0;  // ADC injected sequence register,
    uint32_t RESERVED5[4] = {};
    uint32_t OFR1 = 0;  // ADC offset register 1,
    uint32_t OFR2 = 0;  // ADC offset register 2,
    uint32_t OFR3 = 0;  // ADC offset register 3,
    uint32_t OFR4 = 0;  // ADC offset register 4,
    uint32_t RESERVED6[4] = {};
    uint32_t JDR1 = 0;  // ADC injected data register 1,
    uint32_t JDR2 = 0;  // ADC injected data register 2,
    uint32_t JDR3 = 0;  // ADC injected data register 3,
    uint32_t JDR4 = 0;  // ADC injected data register 4,
    uint32_t RESERVED7[4] = {};
    uint32_t AWD2CR = 0;  // ADC  Analog Watchdog 2 Configuration Register,
    uint32_t AWD3CR = 0;  // ADC  Analog Watchdog 3 Configuration Register,
    uint32_t RESERVED8 = 0;
    uint32_t RESERVED9 = 0;  // Reserved, 0x0AC
    uint32_t DIFSEL = 0;     // ADC  Differential Mode Selection Register,
    uint32_t CALFACT = 0;    // ADC  Calibration Factors,
  };

  struct ADC_Common_TypeDef {
    uint32_t CSR;       // ADC Common status register,
    uint32_t RESERVED;  // Reserved
    uint32_t CCR;       // ADC common control register,
    uint32_t
        CDR;  // ADC common regular data register for dual AND triple modes,
  };

 public:
  ADCRegion(uint32_t base) : MMIORegion(base, 0x400) {}

  uint32_t read(uint32_t offset) final {
    if (offset < 0x300) {  // adc
      ADC_TypeDef* adc = (offset < 0x100) ? &master_adc_ : &slave_adc_;
      auto value = *reinterpret_cast<uint32_t*>(
          reinterpret_cast<uint8_t*>(adc) + (offset & 0xFF));
      // printf("ADC%d: *%08x == %08x (a)\n", index, offset, value);
      return value;
    } else {
      auto value = *reinterpret_cast<uint32_t*>(
          reinterpret_cast<uint8_t*>(&common_adc_) + (offset & 0xFF));
      // printf("ADC%d: *%08x == %08x (c)\n", index, offset, value);
      return value;
    }
  }

  void write_u32(uint32_t offset, uint32_t value) final {
    if (offset == 0x08) {  // CR
      auto changed_fields = master_adc_.CR ^ value;
      if (changed_fields & 0x00000001 and value & 0x00000001) {  // ADEN
        printf("ADC%d: Enable\n", index);
        master_adc_.ISR |= 0x00000001;  // Set ADC Ready
        return;
      }
      if (changed_fields & 0x30000000) {
        printf("ADC%d: Regulator -> %d\n", index, (value & 0x30000000) >> 28);
        master_adc_.CR = value;
        changed_fields &= ~0x30000000;
      }
      if (changed_fields & 0x40000000) {  // adc calibration Mode
        changed_fields &= ~0x40000000;
      }
      if (changed_fields & 0x80000000) {  // adc calibration
        changed_fields &= ~0x80000000;

        if (value & 0x80000000) {
          printf("ADC%d: Calibration request (differential mode: %d)\n", index,
                 (value & 0x40000000) >> 30);
          master_adc_.CR &= ~0x80000000;  // Mark as calibration done.
        } else {
          printf(
              "ADC%d: clearing Calibration bit by software is not allowed.\n",
              index);
        }
      }

      if (changed_fields == 0) {  // successly handled all fields
        return;
      }
      printf("CR remaining fields: %08x\n", changed_fields);
    }
    if (offset == 0x108) {  // CR
      auto changed_fields = master_adc_.CR ^ value;
      if (changed_fields & 0x00000001 and value & 0x00000001) {  // ADEN
        printf("ADC%d: Enable\n", index);
        master_adc_.ISR |= 0x00000001;  // Set ADC Ready
        return;
      }
      if (changed_fields & 0x30000000) {
        printf("ADC%d: Regulator -> %d\n", index, (value & 0x30000000) >> 28);
        master_adc_.CR = value;
        changed_fields &= ~0x30000000;
      }
      if (changed_fields & 0x40000000) {  // adc calibration Mode
        changed_fields &= ~0x40000000;
      }
      if (changed_fields & 0x80000000) {  // adc calibration
        changed_fields &= ~0x80000000;

        if (value & 0x80000000) {
          printf("ADC%d: Calibration request (differential mode: %d)\n", index,
                 (value & 0x40000000) >> 30);
          master_adc_.CR &= ~0x80000000;  // Mark as calibration done.
        } else {
          printf(
              "ADC%d: clearing Calibration bit by software is not allowed.\n",
              index);
        }
      }

      if (changed_fields == 0) {  // successly handled all fields
        return;
      }
      printf("CR remaining fields: %08x\n", changed_fields);
    }

    if (offset < 0x300) {  // adc
      auto adc = (offset < 0x100) ? &master_adc_ : &slave_adc_;
      *reinterpret_cast<uint32_t*>(reinterpret_cast<uint8_t*>(adc) +
                                   (offset & 0xFF)) = value;
      printf("ADC%d: %08x <- %08x\n", index, offset, value);
    } else if (offset < 0x300 + sizeof(ADC_Common_TypeDef)) {
      *reinterpret_cast<uint32_t*>(reinterpret_cast<uint8_t*>(&common_adc_) +
                                   (offset & 0xFF)) = value;
      printf("ADC%d: %08x <- %08x\n", index, offset, value);
    } else {
      printf("ADC%d: %08x <- %08x (out of range)\n", index, offset, value);
    }
    // printf("ADC%d: %08x <- %08x\n", index, offset, value);
    return;
  }

  void reset() override {
    master_adc_ = ADC_TypeDef{};
    slave_adc_ = ADC_TypeDef{};
    common_adc_ = ADC_Common_TypeDef{};
  }

 private:
  ADC_TypeDef master_adc_;
  ADC_TypeDef slave_adc_;
  ADC_Common_TypeDef common_adc_;
};

}  // namespace mcu_emulator::mmio

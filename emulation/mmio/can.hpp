#pragma once

#include <cstdint>
#include <cstdio>

#include "mmio.hpp"

namespace mcu_emulator::mmio {
template <int index>
class CANRegion : public MMIORegion {
  struct CAN_TxMailBox_TypeDef {
    uint32_t TIR;
    uint32_t TDTR;
    uint32_t TDLR;
    uint32_t TDHR;
  };

  struct CAN_FIFOMailBox_TypeDef {
    uint32_t RIR;
    uint32_t RDTR;
    uint32_t RDLR;
    uint32_t RDHR;
  };

  struct CAN_FilterRegister_TypeDef {
    uint32_t FR1;
    uint32_t FR2;
  };

  struct CAN_TypeDef {
    uint32_t MCR;
    uint32_t MSR;
    uint32_t TSR;
    uint32_t RF0R;
    uint32_t RF1R;
    uint32_t IER;
    uint32_t ESR;
    uint32_t BTR;
    uint32_t RESERVED0[88];
    CAN_TxMailBox_TypeDef sTxMailBox[3];
    CAN_FIFOMailBox_TypeDef sFIFOMailBox[2];
    uint32_t RESERVED1[12];
    uint32_t FMR;
    uint32_t FM1R;
    uint32_t RESERVED2;
    uint32_t FS1R;
    uint32_t RESERVED3;
    uint32_t FFA1R;
    uint32_t RESERVED4;
    uint32_t FA1R;
    uint32_t RESERVED5[8];
    CAN_FilterRegister_TypeDef sFilterRegister[28];
  };

 public:
  CANRegion(uint32_t base) : MMIORegion(base, 0x400) {}

  uint32_t read(uint32_t offset) final {
    auto value = *reinterpret_cast<uint32_t*>(
        reinterpret_cast<uint8_t*>(&can_) + offset);
    if (offset == 4)
      return value;
    if (offset == 0x18)
      return value;
    if (offset == 0x188)
      return value;
    if (offset == 0x18C)
      return value;
    if (offset == 0x1A8)
      return value;
    if (offset == 0x1AC)
      return value;
    printf("CAN%d: *%08x == %08x\n", index, offset, value);
    return value;
  }

  void write_u32(uint32_t offset, uint32_t value) final {
    if (offset == 0x00) {    // MCR Access
      if (value & 0x8000) {  // Master Reset
        printf("CAN%d: Master Reset\n", index);
        can_.MCR &= ~0x8000;  // Clear Master Reset
        return;
      }
      if (!(can_.MCR & 0x0001) and value & 0x0001) {  // Initalization request
        printf("CAN%d: Enter Initialization request\n", index);
        can_.MCR |= 0x0001;
        can_.MSR |= (1 << 0);  // Init ACK
        return;
      }
      if (can_.MCR & 0x0001 and !(value & 0x0001)) {  // Leave Init Mode
        printf("CAN%d: Leave initialization request\n", index);
        can_.MCR &= ~0x0001;
        can_.MSR &= ~(1 << 0);  // Init ACK
        return;
      }
      printf("CAN%d: MCR: %08x -> %08x\n", index, can_.MCR, value);
      can_.MCR = value;
      return;
    }

    // range check
    if (offset < sizeof(CAN_TypeDef)) {
      *reinterpret_cast<uint32_t*>(reinterpret_cast<uint8_t*>(&can_) + offset) =
          value;
      printf("CAN%d: %08x <- %08x\n", index, offset, value);
    } else {
      printf("CAN%d: %08x <- %08x (out of range)\n", index, offset, value);
    }
  }

  void reset() override {
    can_.MCR = 0x00010002;
    can_.MSR = 0x00000c02;
    can_.TSR = 0x1c000000;
    can_.BTR = 0x00230000;
    can_.FMR = 0x00000001;
  }

 private:
  CAN_TypeDef can_;
};

}  // namespace mcu_emulator::mmio

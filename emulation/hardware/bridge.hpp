#pragma once

namespace mcu_emulator::hardware {
struct Bridge {
  using Handler = void (*)();
  struct VectorTable {
    Handler entries[0x200 / 4];
  };

  VectorTable* vector_table;

  void Init() { vector_table = nullptr; }
};
Bridge& gEmuBridge = *(Bridge*)0xABCD0000;
}  // namespace mcu_emulator::hardware
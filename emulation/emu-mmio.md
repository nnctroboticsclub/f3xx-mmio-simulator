# Emulator controller (MMIO)

- MappedRange: 0xABCD0000 - 0xABCD03FF (1KiB)

## Register map

| Address    | type            | Name | Description          |
| ---------- | --------------- | ---- | -------------------- |
| 0xABCD0000 | `(void (*)())*` | vtor | Vector table address |

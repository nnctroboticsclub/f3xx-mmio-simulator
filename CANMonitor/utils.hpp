#pragma once

#include <cstddef>
#include <cstdint>

#define NEWLINE "\x1b[0K\n"

const char* FormatHEX(uint8_t const* src, std::size_t length) {
  static char buf[64] = {0};

  auto ptr = buf;
  for (unsigned int i = 0; i < length; i++) {
    *(ptr++) = "0123456789ABCDEF"[src[i] >> 4];
    *(ptr++) = "0123456789ABCDEF"[src[i] & 0x0F];
    *(ptr++) = ' ';
  }
  *ptr = 0;

  return buf;
}
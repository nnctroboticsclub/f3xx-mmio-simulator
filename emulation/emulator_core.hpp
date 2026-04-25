#pragma once

#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <cereal/archives/portable_binary.hpp>
#include <cereal/cereal.hpp>
#include <cereal/types/string.hpp>
#include <cereal/types/vector.hpp>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <fstream>
#include <map>
#include <string>
#include <thread>

namespace mcu_emulator {
struct EndpointMarker {
  enum Kind : uint8_t { kCAN, kUART, kRF };

  Kind kind : 4;
  int id : 4;

  // clang-format off
  auto operator<=> (const EndpointMarker& other) const {
    return std::tie(kind, id) <=> std::tie(other.kind, other.id);
  }
  // clang-format on

  operator uint8_t() const { return (uint8_t)kind << 4 | (uint8_t)id; }

  template <class Archive>
  void serialize(Archive& archive) {
    archive(*(uint8_t*)this);
  }
};
}  // namespace mcu_emulator

namespace std {
template <>
struct hash<mcu_emulator::EndpointMarker> {
  size_t operator()(const mcu_emulator::EndpointMarker& marker) const {
    return std::hash<uint8_t>()(*(uint8_t*)&marker);
  }
};
}  // namespace std

namespace mcu_emulator {

struct DataMessage {
  EndpointMarker marker = {};
  std::vector<uint8_t> data = {};

  DataMessage() = default;

  explicit DataMessage(EndpointMarker marker, std::vector<uint8_t> const& data)
      : marker(marker), data(data) {}

  explicit DataMessage(EndpointMarker marker, std::string data)
      : marker(marker), data(data.begin(), data.end()) {}

  template <class Archive>
  void serialize(Archive& archive) {
    archive(marker, data);
  }
};

class EmulationNetwork {
  auto ReceiveExactly(uint8_t* buffer, size_t length) -> size_t {
    ssize_t ret = 0;
    auto remaining = sizeof(uint16_t);
    auto* ptr = buffer;
    while (remaining > 0) {
      auto bytes_to_receive = std::min(static_cast<size_t>(remaining), length);
      ret = recv(socket_fd_, ptr, bytes_to_receive, 0);
      printf("Trace: recv(%d, %p, %zu) = %zd\n", socket_fd_, ptr,
             bytes_to_receive, ret);
      if (ret <= 0) {
        printf("Error: %zd, errno:%d, fd: %d, this:%p\n", ret, errno,
               socket_fd_, this);
        return 0;
      }
      remaining -= ret;
      ptr += ret;
    }
    return ptr - buffer;
  }
  auto RxThread() {
    static uint8_t buf[1024];

    while (true) {
      if (ReceiveExactly(buf, sizeof(uint16_t)) == 0) {
        printf("Failed to receive message length, Exiting...\n");
        break;
      }
      uint16_t length = *reinterpret_cast<uint16_t*>(buf);
      if (length > sizeof(buf)) {
        throw std::runtime_error("Data length exceeds buffer size");
      }

      if (ReceiveExactly(buf, length) == 0) {
        printf("Failed to receive message data, Exiting...\n");
        break;
      }

      // Deserialize the data
      std::stringstream ss(std::string((char*)buf, length));
      cereal::PortableBinaryInputArchive archive(ss);
      DataMessage message;
      archive(message);

      // Dispatch the message
      auto it = data_callbacks_.find(message.marker);
      if (it != data_callbacks_.end() and bool(it->second)) {
        it->second(message);
        continue;
      }

      printf("DEBUG: Message received with marker: %02x, length: %zu\n",
             message.marker, message.data.size());

      // Fallback to default callback if registered
      if (fallback_callback_) {
        fallback_callback_(message);
        continue;
      }

      printf("No callback registered for marker: %02x\n", message.marker);
    }
  }

 public:
  EmulationNetwork() = default;
  ~EmulationNetwork() {
    if (socket_fd_ != -2) {
      close(socket_fd_);
    }
  }

  auto Connect(int fd) {
    socket_fd_ = fd;
    printf("EmuNet[%p] connected to %d\n", this, socket_fd_);

    std::thread thread([this]() { RxThread(); });
    thread.detach();
  }

  auto Connect(std::string ipc_path) {
    if (is_connected) {
      throw std::runtime_error("Already connected");
    }
    is_connected = true;

    auto client = socket(AF_UNIX, SOCK_STREAM, 0);
    if (client == -1) {
      throw std::runtime_error("Failed to create socket");
    }

    struct sockaddr_un addr;
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, ipc_path.c_str(), sizeof(addr.sun_path) - 1);

    if (connect(client, (struct sockaddr*)&addr, sizeof(addr)) == -1) {
      throw std::runtime_error("Failed to connect to IPC socket");
    }
    Connect(client);
  }

  auto SendMessage(EndpointMarker marker, std::vector<uint8_t> const& data) {
    if (not is_connected) {
      throw std::runtime_error("Not connected");
    }

    DataMessage msg{marker, data};
    std::stringstream ss;
    {
      using Ar = cereal::PortableBinaryOutputArchive;
      auto archive = Ar(ss);
      archive(msg);
    }

    auto length = ss.str().size();
    std::string packet;
    packet.resize(sizeof(uint16_t) + length);
    memcpy(packet.data(), &length, sizeof(uint16_t));
    memcpy(packet.data() + sizeof(uint16_t), ss.str().data(), length);

    send(socket_fd_, packet.data(), packet.size(), 0);
  }

  auto SendMessage(EndpointMarker marker, std::string_view data) {
    SendMessage(marker, std::vector<uint8_t>(data.begin(), data.end()));
  }

  auto RegisterDataCallback(EndpointMarker marker,
                            std::function<void(DataMessage const&)> callback) {
    data_callbacks_[marker] = callback;
  }

  auto RegisterFallbackCallback(
      std::function<void(DataMessage const&)> callback) {
    fallback_callback_ = callback;
  }

 private:
  bool is_connected = false;
  int socket_fd_ = -2;

  std::map<EndpointMarker, std::function<void(DataMessage const&)>>
      data_callbacks_ = {};
  std::function<void(DataMessage const&)> fallback_callback_ = nullptr;
};
}  // namespace mcu_emulator
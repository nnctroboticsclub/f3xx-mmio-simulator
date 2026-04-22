#pragma once

#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <algorithm>
#include <cereal/archives/portable_binary.hpp>
#include <cereal/cereal.hpp>
#include <cereal/types/string.hpp>
#include <cereal/types/vector.hpp>
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
  auto RxThread() {
    static char buf[1024];

    while (true) {
      // receive 2byte length
      auto ret = recv(socket_fd_, buf, sizeof(uint16_t), 0);
      if (ret < 0) {
        throw std::runtime_error("Failed to receive data");
      }
      if (ret != sizeof(uint16_t)) {
        throw std::runtime_error("Invalid data length");
      }
      uint16_t length = *reinterpret_cast<uint16_t*>(buf);
      if (length > sizeof(buf)) {
        throw std::runtime_error("Data length exceeds buffer size");
      }

      // receive data
      auto remaining = length;
      auto ptr = buf;
      while (remaining > 0) {
        auto bytes_to_receive = std::min(size_t(remaining), sizeof(buf));
        ret = recv(socket_fd_, ptr, bytes_to_receive, 0);
        if (ret < 0) {
          printf("Error: %zd, errno:%d, fd: %d, this:%p\n", ret, errno,
                 socket_fd_, this);
          throw std::runtime_error("Failed to receive data");
        }
        remaining -= ret;
        ptr += ret;
      }

      // Deserialize the data
      std::stringstream ss(std::string(buf, length));
      cereal::PortableBinaryInputArchive archive(ss);
      DataMessage message;
      archive(message);

      // Dispatch the message
      auto it = data_callbacks_.find(message.marker);
      if (it != data_callbacks_.end() and bool(it->second)) {
        it->second(message);
        continue;
      }

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

  auto MakeTestPacketDump() {
    DataMessage msg{{
                        .kind = EndpointMarker::kUART,
                        .id = 0x01,
                    },
                    "\x01\x02\x03\x04"};
    std::stringstream ss;
    {
      using Ar = cereal::PortableBinaryOutputArchive;
      auto archive = Ar(ss);
      archive(msg);
    }

    auto data = ss.str();
    uint16_t length = data.size();
    std::string packet;
    packet.resize(sizeof(uint16_t) + length);
    memcpy(packet.data(), &length, sizeof(uint16_t));
    memcpy(packet.data() + sizeof(uint16_t), data.data(), length);

    printf("Packet: ");
    for (const auto& byte : packet) {
      printf("%02x ", byte);
    }
    printf("\n");

    std::ofstream file("dump.bin", std::ios::binary);
    file.write(packet.data(), packet.size());
    file.close();

    printf("Packet written to packet.bin\n");
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
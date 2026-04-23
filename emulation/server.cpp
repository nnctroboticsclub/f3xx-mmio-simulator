#include "emulator_core.hpp"

#include <cereal/archives/json.hpp>

class EmulationClient {
 public:
  EmulationClient(std::function<void(mcu_emulator::DataMessage const&)> handler,
                  int fd) {
    nw_.Connect(fd);
    nw_.RegisterFallbackCallback(handler);
  }

  void SendMessage(mcu_emulator::EndpointMarker marker,
                   std::vector<uint8_t> data) {
    nw_.SendMessage(marker, data);
  }

 private:
  mcu_emulator::EmulationNetwork nw_{};
};

class EmulationServer {
 public:
  EmulationServer() = default;
  ~EmulationServer() { close(server_fd); }

  void Listen(const char* path) {
    server_fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (server_fd < 0) {
      throw std::runtime_error("Failed to create socket");
    }

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, path, sizeof(addr.sun_path) - 1);

    unlink(path);

    if (bind(server_fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
      throw std::runtime_error("Failed to bind socket");
    }

    if (listen(server_fd, 5) < 0) {
      throw std::runtime_error("Failed to listen on socket");
    }

    printf("EmuNet[%p] listening on %s\n", this, path);
  }

  EmulationClient Accept() {
    struct sockaddr_un addr;
    socklen_t addr_len = sizeof(addr);
    int client_fd = accept(server_fd, (struct sockaddr*)&addr, &addr_len);
    if (client_fd < 0) {
      perror("accept");
      exit(EXIT_FAILURE);
    }

    return EmulationClient(
        [this](mcu_emulator::DataMessage const& msg) {
          //
        },
        client_fd);
  }

  auto BroadcastMessage(mcu_emulator::DataMessage const& msg) {
    std::stringstream ss;
    {
      using Ar = cereal::JSONOutputArchive;
      auto archive = Ar(ss, Ar::Options::NoIndent());
      archive(msg);
    }
    std::string buf = ss.str();

    printf("EmuNet[%p] <-- %s\n", this, buf.c_str());

    for (auto& client : clients_) {
      client.SendMessage(msg.marker, msg.data);
    }
  }

  void Loop() {
    while (true) {
      clients_.push_back(std::move(Accept()));
    }
  }

 private:
  int server_fd;
  std::vector<EmulationClient> clients_;
};

static EmulationServer server;
int main() {
  server.Listen("emu.sock");
  if (false) {
    std::thread([&]() {
      auto marker = mcu_emulator::EndpointMarker{
          .kind = mcu_emulator::EndpointMarker::Kind::kUART,
          .id = 1,
      };

      sleep(2);
      server.BroadcastMessage(
          mcu_emulator::DataMessage{marker, "Hello, world!\n"});
      sleep(1);
      server.BroadcastMessage(mcu_emulator::DataMessage{marker, "0123\n"});
      sleep(1);
      server.BroadcastMessage(mcu_emulator::DataMessage{marker, "ABCD\n"});
      sleep(1);
      server.BroadcastMessage(mcu_emulator::DataMessage{marker, "01230123\n"});
      sleep(1);
    }).detach();
  }

  server.Loop();

  return 0;
}
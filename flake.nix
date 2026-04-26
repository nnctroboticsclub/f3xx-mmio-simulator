{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/release-25.11";

  # inputs.roboenv.url = "github:nnctroboticsclub/roboenv-nix";
  inputs.roboenv.url = "path:/home/syoch/ghq/github.com/syoch/libs-dev/libs/roboenv-nix";
  inputs.roboenv.inputs.nixpkgs.follows = "nixpkgs";

  inputs.nano.url = "github:nnctroboticsclub/nano";
  inputs.nano.inputs.roboenv.follows = "roboenv";
  inputs.nano.inputs.nixpkgs.follows = "nixpkgs";

  # inputs.f3-baremetal.url = "github:nnctroboticsclub/f3-baremetal";
  inputs.f3-baremetal.url = "path:/home/syoch/ghq/github.com/syoch/libs-dev/libs/f3-baremetal";
  inputs.f3-baremetal.inputs.roboenv.follows = "roboenv";
  inputs.f3-baremetal.inputs.nano.follows = "nano";
  inputs.f3-baremetal.inputs.nixpkgs.follows = "nixpkgs";

  outputs =
    {
      self,
      nixpkgs,
      roboenv,
      nano,
      f3-baremetal,
    }:
    let
      system = "x86_64-linux";
      rpkgs = roboenv.legacyPackages.${system};
      pkgs = import nixpkgs {
        inherit system;
      };
    in
    {
      packages.${system} = {
        f3xx-mmio-simulator = rpkgs.rlib.buildCMakeProject {
          pname = "f3xx-mmio-simulator";
          version = "v1.0.0";
          src = ./.;

          cmakeBuildInputs = [
            rpkgs.roboenv-loader
            rpkgs.clang-toolchain
            (pkgs.cereal // { cmakeBuildInputs = [ ]; })
          ];
        };
        f3xx-simulated-can-monitor = rpkgs.rlib.buildCMakeProject {
          pname = "f3xx-simulated-can-monitor";
          version = "v1.0.0";
          src = ./CANMonitor;

          cmakeBuildInputs = [
            rpkgs.roboenv-loader
            rpkgs.clang-toolchain
            self.packages.${system}.f3xx-mmio-simulator
            (f3-baremetal.packages.${system}.default.overrideAttrs {
              pname = "f3-baremetal-simulated";
              ROBOPJ_TOOLCHAIN = "ClangToolchain";
              F3BARE_EMULATION = "1";
              F3BARE_USE_STUB_BOOTLOADER = "0";
            })
          ];
        };
        default = self.packages.${system}.f3xx-mmio-simulator;
      };
      devShells.x86_64-linux.default = rpkgs.roboenv {
        name = "f3-baremetal";

        c_cpp.enable = true;
        c_cpp.toolchain = "clang";

        frameworks = [ ];

        extraBuildInputs = pkgs: [
          pkgs.lldb
          pkgs.zydis

          pkgs.cargo
          pkgs.rust-analyzer
          pkgs.pkg-config
          pkgs.udev
          pkgs.rustfmt
          pkgs.rustc
          pkgs.clippy
        ];

        env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

        cmakeInputs = [
          rpkgs.roboenv-loader
          rpkgs.clang-toolchain
          (pkgs.cereal // { cmakeBuildInputs = [ ]; })
          # self.packages.${system}.f3xx-mmio-simulator
          (f3-baremetal.packages.${system}.default.overrideAttrs {
            pname = "f3-baremetal-simulated";
            ROBOPJ_TOOLCHAIN = "ClangToolchain";
            F3BARE_EMULATION = "1";
            F3BARE_USE_STUB_BOOTLOADER = "0";
          })
        ];
      };
    };
}

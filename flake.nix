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

  inputs.devconsole.url = "github:syoch/devconsole";

  inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      roboenv,
      f3-baremetal,
      rust-overlay,
      devconsole,
      ...
    }:
    let
      system = "x86_64-linux";
      rpkgs = roboenv.legacyPackages.${system};
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)
        ];
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
        name = "f3xx-mmio-simulator";

        c_cpp.enable = true;
        c_cpp.toolchain = "clang";

        frameworks = [ ];

        extraBuildInputs = pkgs: [
          pkgs.lldb
          pkgs.zydis

          (pkgs.rust-bin.nightly.latest.default.override {
            targets = [ "x86_64-unknown-none" ];
            extensions = [ "rust-src" ];
          })
          pkgs.rust-analyzer
          pkgs.pkg-config
          pkgs.udev
          pkgs.rustfmt
          pkgs.clippy
          pkgs.gdb

          devconsole.packages.${system}.default
        ];

        env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

        cmakeInputs = [
          rpkgs.roboenv-loader
          rpkgs.clang-toolchain
          (pkgs.cereal // { cmakeBuildInputs = [ ]; })
          # self.packages.${system}.f3xx-mmio-simulator

          (pkgs.stdenv.mkDerivation {
            name = "newlib-cmake";
            src = pkgs.newlib;
            cmakeBuildInputs = [ ];

            buildPhase = ''
              mkdir -p $out/lib/cmake/Newlib
              cat > $out/lib/cmake/Newlib/NewlibConfig.cmake << EOF
              # Newlib CMake configuration file
              set(Newlib_INCLUDE_DIRS "$src/x86_64-unknown-linux-gnu/include")
              set(Newlib_LIB_DIRS "$src/x86_64-unknown-linux-gnu/lib")
              if (NOT TARGET Newlib::Newlib)
                add_library(Newlib::Newlib INTERFACE IMPORTED)
                set_target_properties(Newlib::Newlib PROPERTIES
                  INTERFACE_INCLUDE_DIRECTORIES "\''${Newlib_INCLUDE_DIRS}"
                )

                add_library(Newlib::libc INTERFACE IMPORTED)
                target_link_libraries(Newlib::libc INTERFACE Newlib::Newlib)
                set_target_properties(Newlib::libc PROPERTIES
                  INTERFACE_LINK_LIBRARIES "\''${Newlib_LIB_DIRS}/libc.a"
                )

                add_library(Newlib::libm INTERFACE IMPORTED)
                target_link_libraries(Newlib::libm INTERFACE Newlib::Newlib)
                set_target_properties(Newlib::libm PROPERTIES
                  INTERFACE_LINK_LIBRARIES "\''${Newlib_LIB_DIRS}/libm.a"
                )

                add_library(Newlib::libg INTERFACE IMPORTED)
                target_link_libraries(Newlib::libg INTERFACE Newlib::Newlib)
                set_target_properties(Newlib::libg PROPERTIES
                  INTERFACE_LINK_LIBRARIES "\''${Newlib_LIB_DIRS}/libg.a"
                )

                add_library(Newlib::libnosys INTERFACE IMPORTED)
                target_link_libraries(Newlib::libnosys INTERFACE Newlib::Newlib)
                set_target_properties(Newlib::libnosys PROPERTIES
                  INTERFACE_LINK_LIBRARIES "\''${Newlib_LIB_DIRS}/libnosys.a"
                )
              endif()
              EOF
            '';

          })
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

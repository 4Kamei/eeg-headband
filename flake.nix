{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, utils, rust-overlay} :
    utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        xtask = pkgs.writeShellScriptBin "xtask" ''
            set -euo pipefail

            repo_root="$(${pkgs.git}/bin/git rev-parse --show-toplevel)"

            cd "$repo_root/firmware/xtask"
            cargo r -- "$@"
            cd ..
        '';

      in
      {
        devShell = with pkgs; mkShell rec {
          
          buildInputs = [

            (rust-bin.stable.latest.default.override {
              extensions = ["rust-src"];
              targets = [
                #"aarch64-unknown-linux-gun"     #Darwin?
                "x86_64-unknown-linux-gnu"      #Native linux
                "thumbv8m.main-none-eabi"       #Network core
                "thumbv8m.main-none-eabihf"     #Application core
              ];
            })
            
            xtask
            pkg-config
            gcc
            cargo
            rustc
            rustfmt
            rustPackages.clippy
            dbus
   
            perf

            capnproto

            gdb

            # Linux dependencies for running vulkan
            libxcb
            libxkbcommon
   
            vulkan-loader
            vulkan-headers
            vulkan-tools

            libclang

            rust-analyzer

          ];
    
          LIBCLANG_PATH = "${libclang.lib}/lib";
          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      }
    );
}

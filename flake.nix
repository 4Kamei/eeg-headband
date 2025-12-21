{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, utils, rust-overlay}:
    utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        devShell = with pkgs; mkShell rec {
          
          buildInputs = [

            (rust-bin.stable.latest.default.override {
              extensions = ["rust-src"];
              targets = [
                "thumbv8m.main-none-eabi"       #Network core
                "thumbv8m.main-none-eabihf"     #Application core
              ];
            })
            
            pkg-config
            gcc
            cargo
            rustc
            rustfmt
            rustPackages.clippy
          
            
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

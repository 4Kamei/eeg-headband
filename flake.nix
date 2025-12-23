{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    #crane.url = "github:ipetkov/crane";
    #crane.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, utils, rust-overlay} :
    utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        #srcXtask = ./xtask;
        #cargoArtifacts = craneLib.buildDepsOnly {
        #    src = srcXtask;
        #};
        #
        #xtask_bin = craneLib.buildPackage {
        #    inherit cargoArtifacts
        #    src = srcXtask;
        #};

        xtask = pkgs.writeShellScriptBin "xtask" ''
            set -euo pipefail

            repo_root="$(${pkgs.git}/bin/git rev-parse --show-toplevel)"

            cd "$repo_root/xtask"
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

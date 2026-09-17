{
  description = "Rust dev environment for qutemarks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "qutemarks";
          version = "1.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          buildInputs = with pkgs; [ pkg-config openssl ];
          nativeBuildInputs = with pkgs; [ pkg-config ];
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            rustPackages.clippy
            cargo-watch
            pkg-config
            openssl
          ];
        };
      }
    );
}

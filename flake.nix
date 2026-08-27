{
  description = "Script-friendly EFISTUB manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , rust-overlay
    , crane
    ,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustfmt"
          ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        commonArgs = {
          src = pkgs.lib.cleanSource ./.;
          strictDeps = true;
          buildInputs = [ ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      in
      {
        packages.default = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            meta = {
              description = "Script-friendly EFISTUB manager";
              license = pkgs.lib.licenses.gpl3Only;
              platforms = pkgs.lib.platforms.linux;
              mainProgram = "efistubmgr";
            };
          }
        );

        packages.efistubmgr = self.packages.${system}.default;

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          packages = [
            rustToolchain
            pkgs.rust-analyzer
          ];
        };

        checks = {
          efistubmgr-fmt = craneLib.cargoFmt commonArgs;
        };

        formatter = pkgs.nixpkgs-fmt;
      }
    );
}

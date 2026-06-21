{
  description = "plum";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      packages.${system} = {
        plum-cli = pkgs.rustPlatform.buildRustPackage {
          pname = "plum-cli";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [ "-p" "plum-cli" ];
        };

        default = self.packages.${system}.plum-cli;
      };
    };
}

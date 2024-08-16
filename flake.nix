{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
  let
    systems = builtins.filter
      (system: nixpkgs.lib.strings.hasSuffix "linux" system)
      flake-utils.lib.defaultSystems;
  in {

    overlays.default = final: prev: {
      appearance-watcher = final.callPackage ./package.nix {};
    };
  } // flake-utils.lib.eachSystem systems (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ self.overlays.default ];
      };
    in {
      packages.default = pkgs.appearance-watcher;

      checks = {
        inherit (pkgs) appearance-watcher;
      };

      devShells.default = pkgs.mkShell {
        inputsFrom = [ pkgs.appearance-watcher ];
      };
    }
  );
}

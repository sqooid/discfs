{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
  };
  outputs = { self, nixpkgs, crane, ... }:
    let
      forSystem = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
      pkgsFor = forSystem (system:
        import nixpkgs { inherit system; }
      );
    in
    {
      packages = forSystem
        (system:
          let
            pkgs = pkgsFor."${system}";
            manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
            craneLib = crane.lib.${system};
            sqlFilter = path: _type: null != builtins.match ".*\.sql.*" path;
            sqlOrCargo = path: type: (sqlFilter path type) || (craneLib.filterCargoSources path type);
          in
          {
            discfs = craneLib.buildPackage {

              src = pkgs.lib.cleanSourceWith {
                src = craneLib.path ./.; # The original, unfiltered source
                filter = sqlOrCargo;
              };

              doCheck = false;
              nativeBuildInputs = with pkgs; [ pkg-config ];
              buildInputs = with pkgs; [ openssl.dev sqlite fuse3 ];
            };
            default = self.packages.${system}.discfs;
          }
        );
      devShells = forSystem
        (system:
          let
            pkgs = pkgsFor."${system}";
          in
          {
            default = pkgs.mkShell {
              buildInputs = with pkgs; [
                rustup
                openssl.dev
                pkg-config
                sqlite
                fuse3
              ];
            };
          }
        );
    };
}

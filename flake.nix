{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
  };
  outputs = { self, nixpkgs, ... }:
    let
      forSystem = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        # "aarch64-darwin"
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
          in
          {
            # app = pkgs.callPackage ./path { };
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
                fuse
              ];
            };
          }
        );
    };
}

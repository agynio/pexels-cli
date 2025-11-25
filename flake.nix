{
  description = "Pexels CLI packaged releases";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;

      version = "0.1.1";

      assets = {
        "x86_64-linux" = {
          url = "https://github.com/agynio/pexels-cli/releases/download/v0.1.1/pexels-cli-linux-amd64.tar.gz";
          hash = "sha256-XXeJ3+hFM20jCAT0N+nsf4ZaSt+/TXI8ak1P+DK2xwA=";
        };
        "aarch64-linux" = {
          url = "https://github.com/agynio/pexels-cli/releases/download/v0.1.1/pexels-cli-linux-arm64.tar.gz";
          hash = "sha256-vYnYKhYEFTTvlcdHcTOBZ5sUxfwThJVVd71V2GqiG3o=";
        };
        "x86_64-darwin" = {
          url = "https://github.com/agynio/pexels-cli/releases/download/v0.1.1/pexels-cli-macos-amd64.tar.gz";
          hash = "sha256-gvWaFz/bclIxcNNzOCTcrI89Pq3BUTyq47Lfr4aUxCw=";
        };
        "aarch64-darwin" = {
          url = "https://github.com/agynio/pexels-cli/releases/download/v0.1.1/pexels-cli-macos-arm64.tar.gz";
          hash = "sha256-T08ffQjgZEElyPG51aaIT/byNTy1nn6lDScyTISpou4=";
        };
      };
    in {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          asset = assets.${system};
        in {
          default = pkgs.stdenvNoCC.mkDerivation {
            pname = "pexels-cli";
            inherit version;

            src = pkgs.fetchurl {
              inherit (asset) url hash;
            };

            dontUnpack = true;
            dontConfigure = true;
            dontBuild = true;

            installPhase = ''
              runHook preInstall
              tar -xzf "$src"
              install -Dm755 pexels "$out/bin/pexels"
              runHook postInstall
            '';

            meta = with pkgs.lib; {
              mainProgram = "pexels";
              description = "Pexels CLI";
              homepage = "https://github.com/agynio/pexels-cli";
              license = licenses.mit;
              platforms = systems;
            };
          };
        });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/pexels";
          meta = {
            description = "Run the Pexels CLI";
          };
        };
      });
    };
}

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
          hash = "sha256-L1RDY8EYBqaeAV69QHP9CbzrgL7gjku6iefPrhML/p4=";
        };
        "aarch64-linux" = {
          url = "https://github.com/agynio/pexels-cli/releases/download/v0.1.1/pexels-cli-linux-arm64.tar.gz";
          hash = "sha256-i2WS3l35hFsHo/RU5l3xNxMNebaamvzsqwovODeBow0=";
        };
        "x86_64-darwin" = {
          url = "https://github.com/agynio/pexels-cli/releases/download/v0.1.1/pexels-cli-macos-amd64.tar.gz";
          hash = "sha256-t2VgHEDz7lSI7RLqHi8nwzGzDDbbnPuxAkHBCLakIA8=";
        };
        "aarch64-darwin" = {
          url = "https://github.com/agynio/pexels-cli/releases/download/v0.1.1/pexels-cli-macos-arm64.tar.gz";
          hash = "sha256-6KJmpvcU0DVnCKwve4g9OlAgMwtVwfQ1T/w8quvbGpg=";
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

            src = pkgs.fetchzip {
              inherit (asset) url hash;
            };

            installPhase = ''
              install -Dm755 pexels "$out/bin/pexels"
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

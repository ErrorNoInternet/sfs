{
  description = "fschool-agent development shell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    mozilla.url = "github:mozilla/nixpkgs-mozilla";
  };

  outputs = { self, nixpkgs, mozilla }:
  let
    system = "x86_64-linux";
    overlays = [ self.inputs.mozilla.overlays.rust ];
    pkgs = import nixpkgs { inherit overlays system; };
    pkgsStatic = pkgs.pkgsStatic;
    pkgsCross = pkgs.pkgsCross;
    channel = pkgs.rustChannelOf {
      channel = "stable";
      sha256 = "sha256-rLP8+fTxnPHoR96ZJiCa/5Ans1OojI7MLsmSqR2ip8o=";
    };
    rust = (channel.rust.override {
      targets = [
        "x86_64-unknown-linux-gnu"
        "x86_64-unknown-linux-musl"
        "i686-unknown-linux-musl"
      ];
      extensions = [ "rust-src" ];
    });
  in
  {
    devShells.${system}.default = pkgs.mkShell {
      name = "rust-environment";
      nativeBuildInputs = with pkgs; [
        pkg-config
        openssl
      ];
      buildInputs = [
        rust
      ];

      LIBZ_SYS_STATIC = 1;
      OPENSSL_DIR = pkgsStatic.openssl.dev;
      OPENSSL_LIB_DIR = "${pkgsStatic.openssl.out}/lib";
      OPENSSL_STATIC = 1;
      PKG_CONFIG_ALL_STATIC = true;
      PKG_CONFIG_ALLOW_CROSS = true;
    };
  };
}

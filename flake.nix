{
  description = "parlang: A programming language project";

  inputs = {
    rust-overlay.url = "https://flakehub.com/f/oxalica/rust-overlay/0.1.2054";
    flake-utils.url = "https://flakehub.com/f/numtide/flake-utils/0.1.102";
    nixpkgs.follows = "rust-overlay/nixpkgs";
  };

  outputs =
    inputs:
    with inputs;
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;

        rustVersion = "latest";
        rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        parlang = rustPlatform.buildRustPackage {
          pname = manifest.name;
          version = manifest.version;
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl.dev ];
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        };
      in
      rec {
        packages = {
          parlang = parlang;
          default = packages.parlang;
        };

        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.cargo-deny
            pkgs.cargo-edit
            pkgs.cargo-tarpaulin
            pkgs.cargo-watch
            pkgs.cargo-outdated
            pkgs.cargo-update
            pkgs.git
            pkgs.openssl
            pkgs.pkg-config
            pkgs.rust-analyzer
          ];

          env = {
            # Required by rust-analyzer
            RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          };
        };
      }
    );
}

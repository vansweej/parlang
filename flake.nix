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
          manifest = (pkgs.lib.importTOML ./crates/parlang/Cargo.toml).package;

          # A Rust bump requires updating the overlay and re-running the memory
          # measurement protocol because compiler memory behavior can change.
          rustVersion = "1.93.1";
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

            # editor_grammar_tests run the real tree-sitter (generate needs
            # node) and bat CLIs and fail instead of skipping without them.
            nativeCheckInputs = [ pkgs.bat pkgs.nodejs pkgs.tree-sitter ];
            preCheck = ''
              export HOME="$TMPDIR"
              export XDG_CACHE_HOME="$TMPDIR/.cache"
            '';

            meta.mainProgram = "parlang";
          };

          treeSitterParlang = pkgs.tree-sitter.buildGrammar {
            language = "parlang";
            version = manifest.version;
            src = ./editors/tree-sitter-parlang;
            generate = true;
          };

          parlangNvim = pkgs.linkFarm "parlang-nvim" [
            { name = "parser/parlang.so"; path = "${treeSitterParlang}/parser"; }
            { name = "queries/parlang/highlights.scm"; path = ./editors/tree-sitter-parlang/queries/highlights.scm; }
            { name = "ftdetect/parlang.lua"; path = ./editors/nvim/ftdetect/parlang.lua; }
            { name = "ftplugin/parlang.lua"; path = ./editors/nvim/ftplugin/parlang.lua; }
          ];

          parlangBatSyntax = pkgs.linkFarm "parlang-bat-syntax" [
            { name = "ParLang.sublime-syntax"; path = ./editors/sublime/ParLang.sublime-syntax; }
          ];

          # The ParLang book (book/): pandoc + LuaLaTeX, fonts for the sandbox.
          # Font leaf directories (no subdirectory search needed): texmf.cnf appends
          # // only after the whole expanded $OSFONTDIR, so a recursive root would
          # only work for the last entry.
          bookFontDirs = pkgs.lib.concatStringsSep ":" [
            "${pkgs.dejavu_fonts}/share/fonts/truetype"
            "${pkgs.noto-fonts-cjk-sans}/share/fonts/opentype/noto-cjk"
            "${pkgs.noto-fonts-monochrome-emoji}/share/fonts/noto"
          ];
          bookTex = pkgs.texliveMedium.withPackages (ps: with ps; [ latexmk fvextra lineno upquote ]);

          # The book only runs the interpreter; its tests run in packages.parlang.
          parlangForBook = parlang.overrideAttrs (_: { doCheck = false; });

          parlangBook = pkgs.stdenvNoCC.mkDerivation {
            pname = "parlang-book";
            version = manifest.version;
            src = ./.;

            nativeBuildInputs = [ bookTex pkgs.pandoc parlangForBook ];
            OSFONTDIR = bookFontDirs;
            dontConfigure = true;

            buildPhase = ''
              runHook preBuild
              export HOME="$TMPDIR"
              bash book/build-book
              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              mkdir -p "$out"
              cp book/build/parlang-book.pdf "$out/parlang-book.pdf"
              cp book/build/parlang-book.epub "$out/parlang-book.epub"
              cp book/build/glyph-test.pdf "$out/glyph-test.pdf"
              runHook postInstall
            '';
          };

          # Lint for the hand-written book sources; cheap, unlike the book
          # build itself, which is not a check.
          bookLint = pkgs.runCommand "parlang-book-lint"
            {
              nativeBuildInputs = [ bookTex pkgs.shellcheck pkgs.shfmt ];
            }
            ''
              cd ${./book}
              shellcheck build-book
              shfmt -i 2 -ci -sr -d build-book
              shellcheck tools/render-diagrams
              shfmt -i 2 -ci -sr -d tools/render-diagrams
              chktex --version > /dev/null
              chktex -q -v0 -I0 -l .chktexrc preamble.tex glyph-test.tex parlang-book.tex > "$TMPDIR/chktex.out" 2> "$TMPDIR/chktex.err" || true
              if grep -Eq '^[^:]+:[0-9]+:[0-9]+:' "$TMPDIR/chktex.out"; then
                cat "$TMPDIR/chktex.out" "$TMPDIR/chktex.err"
                exit 1
              fi
              touch "$out"
            '';
        in
        rec {
          packages = {
            parlang = parlang;
            default = packages.parlang;
            tree-sitter-parlang = treeSitterParlang;
            parlang-nvim = parlangNvim;
            parlang-bat-syntax = parlangBatSyntax;
            parlang-book = parlangBook;
          };

          devShells.default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.bat
              pkgs.cargo-deny
              pkgs.cargo-edit
              pkgs.cargo-tarpaulin
              pkgs.cargo-watch
              pkgs.cargo-outdated
              pkgs.cargo-update
              pkgs.git
              pkgs.nixpkgs-fmt
              pkgs.nodejs
              pkgs.openssl
              pkgs.pkg-config
              pkgs.rust-analyzer
              pkgs.tree-sitter
            ];

            env = {
              # Required by rust-analyzer
              RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
              PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            };
          };

          devShells.book = pkgs.mkShell {
            packages = [ bookTex pkgs.pandoc parlangForBook ];

            env = {
              OSFONTDIR = bookFontDirs;
            };
          };

          checks = {
            inherit (packages) tree-sitter-parlang parlang-nvim parlang-bat-syntax;
            book-lint = bookLint;
          };
        }
      );
}

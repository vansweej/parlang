# Using the nix flake

The repository ships a `flake.nix` that pins every tool the project
needs: the Rust toolchain, the editor grammar tools, and the TeX
setup that typesets this book. With nix and flakes enabled you can
build, run and develop ParLang without installing anything else.

The flake uses `eachDefaultSystem`, so its outputs exist for x86_64
and aarch64 on Linux and macOS. The project only tests x86_64-linux.

## What the flake provides

| Output | What it is |
|--------|------------|
| `packages.<system>.parlang` | The interpreter (also `default`) |
| `packages.<system>.tree-sitter-parlang` | The tree-sitter grammar |
| `packages.<system>.parlang-nvim` | A Neovim plugin directory |
| `packages.<system>.parlang-bat-syntax` | A syntax file for bat |
| `packages.<system>.parlang-book` | This book as PDF and EPUB |
| `devShells.<system>.default` | The Rust development shell |
| `devShells.<system>.book` | The shell for building the book |
| `checks.<system>.*` | Editor packages and the book lint |

The `parlang` package sets `meta.mainProgram` to `parlang`, so
`nix run` knows which binary to start. Building it runs the tests of
the `parlang` crate (the package builds only the default workspace
member), including the editor grammar tests, with bat, node and
tree-sitter available in the build sandbox.

Each editor package produces a small result directory:

- `tree-sitter-parlang`: `parser` and `queries/highlights.scm`.
- `parlang-nvim`: `parser/parlang.so`,
  `queries/parlang/highlights.scm`, `ftdetect/parlang.lua` and
  `ftplugin/parlang.lua`.
- `parlang-bat-syntax`: `ParLang.sublime-syntax`.
- `parlang-book`: `parlang-book.pdf`, `parlang-book.epub` and
  `glyph-test.pdf`.

The default development shell provides the Rust 1.93.1 toolchain
with cargo, clippy, rustfmt and rust-analyzer, plus tree-sitter, bat,
node, git, nixpkgs-fmt, cargo-tarpaulin, cargo-deny, cargo-edit,
cargo-watch, cargo-outdated and cargo-update. The `book` shell
provides TeX Live 2024 with LuaLaTeX and latexmk, pandoc 3.1 and the
interpreter on `PATH`. It also sets `OSFONTDIR` to the flake's fonts.

To list every output:

```bash
nix flake show
```

## Building and running the interpreter

Build the default package. The binary ends up in
`result/bin/parlang`:

```bash
nix build
nix build .#parlang
```

Both commands build the same package. Because the build runs the
tests, the first build takes a while.

`nix run` builds the interpreter if needed and starts it. Arguments
after `--` go to `parlang`. With no file argument you get the REPL:

```bash
nix run .
```

Pass a file to run it. This prints the program's value:

```bash
nix run . -- crates/parlang/examples/simple.par
```

Show the command-line help or the version:

```bash
nix run . -- --help
nix run . -- --version
```

The chapter on the command-line interface and REPL describes every
option.

Example programs that `load` other files, such as
`load "examples/stdlib.par"`, use paths relative to `crates/parlang`.
Run those programs from inside `crates/parlang`.

## Developing

The project builds and tests in a memory-limited user scope, with a
separate target directory, because compiling the test crates can use
a lot of memory:

```bash
systemd-run --user --scope -p MemoryAccounting=1 \
  -p MemoryMax=70% -p MemorySwapMax=0 \
  env CARGO_TARGET_DIR=/tmp/parlang-cargo \
  nix develop . --command cargo test --workspace
```

On a machine with plenty of memory the plain form works too:

```bash
nix develop
cargo build
cargo test --workspace
```

Always pass `--workspace` to cargo test and clippy. The workspace's
`default-members` only covers `crates/parlang`, so a bare cargo
command silently skips `parlang-core` and `parlang-driver`.

## Editor support

Build the editor packages one at a time:

```bash
nix build .#parlang-nvim
nix build .#parlang-bat-syntax
nix build .#tree-sitter-parlang
```

The Editor Support chapter explains how to set up Neovim and bat
with these results.

## Building this book

Flakes only see files that git tracks, so `git add` new files first;
changes to tracked files are picked up without staging. Then build
the package:

```bash
git add book
nix build .#parlang-book
```

For quicker iterations, run the build script inside the `book`
shell. It writes its output to `book/build/`:

```bash
nix develop .#book --command bash book/build-book
```

## Checks

```bash
nix flake check
```

For the current system, the checks build the three editor packages
and run `book-lint`, which runs shellcheck, shfmt and chktex on the
book sources. Other systems need `--all-systems`. `nix flake check`
does not build the interpreter's tests or the book. Use `nix build`
and `nix build .#parlang-book` for those.

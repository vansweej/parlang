# The ParLang book

A printable (PDF) and e-reader (EPUB) reference for ParLang, generated
from the repository.

## Building

- Nix: run `git add book` first (flakes only see files tracked by git), then
  `nix build .#parlang-book`. The result contains `parlang-book.pdf`,
  `parlang-book.epub` and `glyph-test.pdf`. `nix flake check` does not
  build the book.
- Interactive: `nix develop .#book --command bash book/build-book` writes
  the same files to `book/build/`. Set `PARLANG` to choose the interpreter
  command, for example
  `PARLANG="cargo run --quiet --package parlang --bin parlang --"`.
- The build script `build-book` has no `.sh` extension on purpose, because
  the ai-coding pipeline routes `.sh` files to a shfmt/shellcheck toolchain
  it does not provide. It is linted instead by the flake check `book-lint`
  (`nix flake check`), which runs shellcheck, `shfmt -i 2 -ci -sr -d` and
  chktex with `book/.chktexrc` on `preamble.tex`, `glyph-test.tex` and
  `parlang-book.tex`.

## How it works

- `chapters.txt` lists parts and chapters in order (paths are relative to
  the repository root).
- `build-book` runs the real interpreter to produce the dumps used by
  `front/reduction-walkthrough.md`, generates the example-programs appendix
  from `crates/parlang/examples/`, builds the glyph coverage test, converts
  every source with pandoc and `filters/parlang.lua`, and produces the PDF
  (`latexmk -lualatex`) and the EPUB (`pandoc --to epub3`).
- Hand-written parts are pandoc Markdown in `front/`. Listings there use
  include fences such as `{.parlang include="crates/parlang/examples/mvp.par"}`
  and must point at files the test suite or `build-book` executes.
- Do not add `.par` files under `book/`: the editor grammar tests walk the
  whole workspace for them.

## Glyph coverage

`build-book` lists every non-ASCII character used by the sources, prints
them in each font in `glyph-test.pdf` and compiles that test before the
book. A character missing from every font stops the build. While the book
is being set up the table is also an appendix; remove the `glyphs` line
from `chapters.txt` to drop it.

## Build checks

A clean build is the test: the build fails on missing glyphs, undefined
references or citations, and rerun warnings left after latexmk. Overfull
boxes are reported as a warning with their count; review them.

Fonts come only from the flake: `luaotfload.conf` stops luaotfload from
scanning system fonts, so a local `nix develop .#book` build uses exactly
the fonts of `nix build .#parlang-book`.

## Adding a chapter

Add a `doc|docs/NAME.md` line to `chapters.txt` in the right part. The
first level-one heading of the file becomes the chapter title.

## Follow-ups

Rendering mermaid diagrams, syntax highlighting for ParLang listings and a
chapter on using the nix flake (build, develop, run) are not part of this
edition.

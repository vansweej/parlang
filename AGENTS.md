# AGENTS.md

ParLang: a small ML-alike functional language interpreter in Rust (single crate,
edition 2021). Parser built on the `combine` combinator library; optional
Hindley-Milner (Algorithm W) type inference.

## Build & test

A `flake.nix` is present — prefix all commands with the Nix dev shell, which
pins the Rust toolchain and provides tarpaulin/clippy/etc:

```bash
nix develop . --command <cmd>
```

| Task | Command |
|------|---------|
| Build | `cargo build` |
| Test all | `cargo test` |
| Single test | `cargo test <test_name>` |
| Single test file | `cargo test --test <file_stem>` (e.g. `--test record_tests`) |
| Lint | `cargo clippy --all-targets --all-features` |
| Format | `cargo fmt` |
| Coverage | `cargo tarpaulin` |

**Pre-PR order:** `cargo clippy` → `cargo fmt` → `cargo test`.

## Toolchain quirks

- Clippy runs with `pedantic` warnings enabled (`Cargo.toml` `[lints.clippy]`).
  `module_name_repetitions` and `must_use_candidate` are allowed; keep all other
  pedantic lints clean.
- `#![recursion_limit = "512"]` in `src/lib.rs` is required by the `combine`
  parser — do not lower it.
- Both a binary (`parlang`, `src/main.rs`) and a library (`src/lib.rs`) target
  exist. The library re-exports the full public API from `src/lib.rs`.

## Running the interpreter

```bash
cargo run                                       # REPL
cargo run -- examples/simple.par               # run a file
cargo run -- examples/simple.par --dump-ast ast.dot  # dump AST (Graphviz DOT)
PARLANG_TYPECHECK=1 cargo run                  # enable HM type checking
```

Type checking is **off by default**. The `PARLANG_TYPECHECK=1` env var affects
both REPL/CLI output and the expected output of many integration tests.

## Module map (`src/`)

| File | Role |
|------|------|
| `ast.rs` | Expression AST (`Expr`, `BinOp`) |
| `parser.rs` | `combine`-based parser; entrypoint `parse()` |
| `types.rs` | Type representations (`Type`, `TypeScheme`, `TypeVar`, `RowVar`) |
| `typechecker.rs` | Algorithm W inference; entrypoint `typecheck()` |
| `exhaustiveness.rs` | Pattern-match exhaustiveness checker (`check_exhaustiveness`) |
| `eval.rs` | Tree-walking interpreter; `eval()`, `Value`, `Environment` |
| `dot.rs` | AST → Graphviz DOT |
| `main.rs` | `clap` CLI + `rustyline` REPL |

Note: `exhaustiveness.rs` is a real public module (re-exported in `lib.rs`) but
is omitted from the README's architecture section.

## Testing conventions

- Unit tests live in-file under `#[cfg(test)] mod tests`.
- Integration tests are per-feature files in `tests/` (e.g. `sum_type_tests.rs`,
  `row_polymorphism_tests.rs`). Run a single file with `cargo test --test <stem>`.
- `.par` files in `tests/` (e.g. `map_assoc_test.par`) are fixtures loaded by
  tests, not standalone test runners.
- Doc examples in `src/lib.rs` are compiled and run by `cargo test` — keep them
  valid.

## Style (from CONTRIBUTING.md)

- **No `unwrap()`/`panic!` in library code.** Use dedicated error variants
  (`EvalError`, `ParseError`, `TypeError`). Panics are acceptable only in tests,
  CLI code, or at invariant assertions (comment why).
- Use **checked arithmetic** (`checked_add`, etc.) for any user-provided values.
- Use `attempt()` in parsers sparingly — it disables the `combine` fast path.
- Prefer pattern matching and early-return `?` over nested `if-let` chains.
- Prefer dedicated error enum variants over generic string errors.

## References

- `README.md` — language syntax and feature reference
- `CONTRIBUTING.md` — full code-quality and PR guidelines
- `docs/ARCHITECTURE.md` — component interaction and design patterns
- `docs/` — type system, per-module deep dives, and example guides

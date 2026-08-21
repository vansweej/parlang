# AGENTS.md

ParLang: a small ML-alike functional language interpreter in Rust (a 3-crate
Cargo workspace: `crates/parlang` for the surface language—parser, typechecker,
tree-walking evaluator, and CLI/REPL; `crates/parlang-core` for the Core IR; and
`crates/parlang-driver` for the runner binary), edition 2021. Parser built on
the `combine` combinator library; Hindley-Milner (Algorithm W) type inference.

## Build & test

A `flake.nix` is present — prefix all commands with the Nix dev shell, which
pins the Rust toolchain and provides tarpaulin/clippy/etc:

```bash
nix develop . --command <cmd>
```

Run build and test commands in a memory-contained user scope. `MemoryMax=70%`
leaves headroom for the login session while `CARGO_TARGET_DIR` keeps contained
build artefacts separate:

```bash
systemd-run --user --scope -p MemoryAccounting=1 -p MemoryMax=70% -p MemorySwapMax=0 \
  env CARGO_TARGET_DIR=/tmp/parlang-cargo nix develop . --command cargo test
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
Run each command through the contained invocation above.

## Toolchain quirks

- Clippy runs with `pedantic` warnings enabled (`Cargo.toml` `[lints.clippy]`).
  `module_name_repetitions` and `must_use_candidate` are allowed; keep all other
  pedantic lints clean.
- `#![recursion_limit = "512"]` in `crates/parlang/src/lib.rs` is required by the `combine`
  parser — do not lower it.
- The `parlang` binary (`crates/parlang/src/main.rs`) and library
  (`crates/parlang/src/lib.rs`) targets exist, alongside the `parlang-driver`
  runner binary (`crates/parlang-driver/src/main.rs`). The library re-exports
  the full public API from `crates/parlang/src/lib.rs`.

## Running the interpreter

```bash
cargo run                                       # REPL
cargo run -- crates/parlang/examples/simple.par               # run a file
cargo run -- crates/parlang/examples/simple.par --dump-dot > ast.dot  # dump AST as DOT to stdout
cargo run                                       # HM type checking runs automatically
```

Type checking is **mandatory**: Hindley-Milner inference runs before evaluation
in both the REPL and file/CLI modes, and shapes the expected output of many
integration tests.

## Module map (`crates/parlang/src/`)

| File | Role |
|------|------|
| `ast.rs` | Expression AST (`Expr`, `BinOp`) |
| `parser.rs` | `combine`-based parser; entrypoints `parse_expr()` / `parse_program()` |
| `types.rs` | Type representations (`Type`, `TypeScheme`, `TypeVar`) |
| `typechecker.rs` | Algorithm W inference; entrypoint `typecheck()` |
| `exhaustiveness.rs` | Pattern-match exhaustiveness checker (`check_exhaustiveness`) |
| `eval.rs` | Tree-walking interpreter; `eval()`, `Value`, `Environment` |
| `dot.rs` | AST → Graphviz DOT |
| `main.rs` | `clap` CLI + `rustyline` REPL |

`crates/parlang-core/src/` holds the Core IR (`base_type.rs`, `term.rs`,
`builder.rs`, `eval.rs`, `prelude.rs`, `display.rs`, `dot.rs`, `error.rs`, and
`lib.rs`); `crates/parlang-driver/src/` contains the runner binary.

## Testing conventions

- Unit tests live in-file under `#[cfg(test)] mod tests`.
- Each integration-test file in `tests/` is a separate crate that links the full
  `combine` parser stack. Prefer extending an existing feature test file over
  adding a new crate, since each additional file multiplies compile-time memory.
  Run a single file with `cargo test --test <stem>` through the contained wrapper.
- `.par` files in `tests/` (e.g. `map_assoc_test.par`) are fixtures loaded by
  tests, not standalone test runners.
- Doc examples in `src/lib.rs` are compiled and run by `cargo test` — keep them
  valid.

## Style (from CONTRIBUTING.md)

- **No `unwrap()`/`panic!` in library code.** Use dedicated error variants
  (`EvalError`, `TypeError`). Panics are acceptable only in tests,
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

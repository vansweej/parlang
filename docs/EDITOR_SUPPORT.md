# Editor Support

There are two highlight-oriented grammars under `editors/`: a tree-sitter
grammar for Neovim and a sublime-syntax grammar for `bat`.

## Neovim

The flake package `parlang-nvim` provides `parser/parlang.so`,
`queries/parlang/highlights.scm`, an ftdetect mapping `*.par` to filetype
`parlang`, and an ftplugin that calls `vim.treesitter.start`; requires
Neovim 0.9+.

Untested home-manager example:

```nix
programs.neovim.plugins = [ inputs.parlang.packages.${pkgs.system}.parlang-nvim ];
```

## bat

The flake package `parlang-bat-syntax` provides the sublime-syntax grammar
for `bat`.

Untested home-manager example:

```nix
programs.bat.syntaxes.parlang = {
  src = inputs.parlang.packages.${pkgs.system}.parlang-bat-syntax;
  file = "ParLang.sublime-syntax";
};
```

Without home-manager one copies the syntax into bat's config syntaxes
directory and runs `bat cache --build`.

## How the grammars are tested

`crates/parlang/tests/editor_grammar_tests.rs`, run under `cargo test` inside
`nix develop`, verifies: every `.par` file parses with the `combine` parser;
`tree-sitter generate` plus a parse of every `.par` file yields no ERROR or
MISSING nodes; highlight captures via a tree-sitter query over
`editors/samples/highlight_sample.par`; bat colours verified via the
test-only theme `editors/sublime/test/ParLangScopes.tmTheme`; both grammars
checked against `parlang::parser::KEYWORDS`; the tests fail (rather than
skip) if `tree-sitter`, `node`, or `bat` is missing; and `nix flake check`
additionally builds the tree-sitter grammar package.

## Limitations

- Flat token grammars rather than a real parse (no folds or text objects).
- ASCII identifiers only.
- The `combine` parser is the specification.
- Both grammars must be updated when syntax changes (keyword-sync and
  corpus tests catch new keywords and unparseable files).

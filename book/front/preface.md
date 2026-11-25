# Preface

This is the ParLang book: a reference for ParLang, a small ML-like
functional language with Hindley-Milner type inference, a tree-walking
surface evaluator and an explicitly typed System F Core that runs on a
CEK machine.

The book is a living document. Almost every chapter is generated from the
Markdown documentation in the `docs/` directory of the repository, and
every program listing outside those chapters is a file that the test suite
or the book build itself executes. When the language changes, the
documentation changes, and the next build of the book follows.

Part I is the language reference. Part II follows a program from the
surface syntax through the typed AST and System F Core down to a trace of
the CEK machine, using real output of the interpreter. Part III covers the
command line, the REPL, the dump stages and editor support. Part IV
documents the standard library. The appendices contain every example
program shipped with the interpreter and the architecture decision
records.

Mermaid diagrams from the Markdown documentation are not rendered in this
edition; a note marks each place where one was omitted.

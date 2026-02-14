# ParLang Documentation

Welcome to the comprehensive documentation for ParLang, a small ML-alike functional language written in Rust.

## 📚 Table of Contents

### Getting Started
- [Main README](../README.md) - Quick start guide and basic usage
- [Examples Guide](EXAMPLES.md) - Tutorial-style examples from basic to advanced
- [Language Specification](LANGUAGE_SPEC.md) - Formal language specification

### Core Documentation
- [Architecture Guide](ARCHITECTURE.md) - System architecture and design
- [Type System](TYPE_SYSTEM.md) - Hindley-Milner type inference
- [Sum Types](SUM_TYPES.md) - Algebraic data types and pattern matching
- [Exhaustiveness Checking](EXHAUSTIVENESS_CHECKING.md) - Complete pattern match verification
- [Generic Types](GENERIC_TYPES.md) - Parameterized types and type inference
- [Records](RECORDS.md) - Product types with named fields
- [API Reference](API_REFERENCE.md) - Complete API documentation for library users

### Module Documentation
- [AST Module](MODULE_AST.md) - Abstract syntax tree data structures
- [Parser Module](MODULE_PARSER.md) - Parser implementation using combinators
- [Evaluator Module](MODULE_EVAL.md) - Expression evaluation and runtime
- [Main Module](MODULE_MAIN.md) - CLI and REPL interface

## 📖 Documentation Overview

### [Architecture Guide](ARCHITECTURE.md)
**For**: Developers, contributors, architects

Comprehensive overview of ParLang's architecture including:
- System architecture with component diagrams
- Data flow and component interaction
- Module structure and dependencies
- Design decisions and trade-offs
- Performance characteristics
- Extension points

**Mermaid Diagrams**: 8 diagrams covering system architecture, data flow, testing strategy, and more

### [Language Specification](LANGUAGE_SPEC.md)
**For**: Language users, implementers, researchers

Formal specification of the ParLang language including:
- Complete lexical structure and syntax
- Formal grammar in BNF/EBNF notation
- Type system and semantics
- Operational semantics with formal rules
- Operator precedence and associativity
- Scoping rules and evaluation order
- Comparison with other functional languages

**Mermaid Diagrams**: 13 diagrams including syntax railroad diagrams, type hierarchy, and evaluation flows

### [Examples Guide](EXAMPLES.md)
**For**: New users, learners, developers

Tutorial-style guide with 80+ examples covering:
- Progressive learning path from basics to advanced
- All language features with explanations
- Common patterns and idioms
- Real-world use cases
- Best practices
- Common mistakes to avoid
- Reference to example `.par` files

### [API Reference](API_REFERENCE.md)
**For**: Library users, integrators

Complete API documentation including:
- Public API exports
- All types with examples
- Function documentation
- Usage patterns
- Error handling guide
- Integration examples
- Embedding ParLang in Rust applications

## 🔧 Module Documentation

### [AST Module](MODULE_AST.md)
**Lines**: ~365 | **Exports**: `Expr`, `BinOp`

Documents the Abstract Syntax Tree data structures:
- Expression types and variants
- Binary operators
- Display trait implementation
- AST construction examples
- Design considerations

**Mermaid Diagrams**: Expression tree visualizations, type hierarchies

### [Parser Module](MODULE_PARSER.md)
**Lines**: ~718 | **Exports**: `parse()` function

Documents the parser implementation:
- Parser combinator approach
- Atomic and composite parsers
- Operator precedence handling
- Error handling
- Testing strategy
- 67 unit tests

**Mermaid Diagrams**: Parser architecture, parsing pipeline, precedence hierarchy

### [Evaluator Module](MODULE_EVAL.md)
**Lines**: ~856 | **Exports**: `eval()`, `Value`, `Environment`, `EvalError`

Documents the interpreter/evaluator:
- Runtime values and types
- Environment management
- Evaluation algorithm
- Closure semantics
- Error handling
- 73 unit tests

**Mermaid Diagrams**: Evaluation flows, closure handling, environment structure

### [Main Module](MODULE_MAIN.md)
**Lines**: ~86 | **Exports**: Binary executable

Documents the CLI/REPL interface:
- REPL mode (interactive)
- File execution mode
- Command-line argument handling
- Error reporting
- User interaction patterns

**Mermaid Diagrams**: CLI flows, REPL loop, error handling

## 📊 Documentation Statistics

| Document | Lines | Size | Diagrams | Topics |
|----------|-------|------|----------|--------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | 385 | 12KB | 8 | System design |
| [LANGUAGE_SPEC.md](LANGUAGE_SPEC.md) | 1,621 | 37KB | 13 | Language reference |
| [EXAMPLES.md](EXAMPLES.md) | 1,226 | 20KB | 0 | 80+ examples |
| [API_REFERENCE.md](API_REFERENCE.md) | 1,299 | 28KB | 0 | Complete API |
| [MODULE_AST.md](MODULE_AST.md) | 639 | 16KB | 5 | AST structures |
| [MODULE_PARSER.md](MODULE_PARSER.md) | 782 | 18KB | 5 | Parser implementation |
| [MODULE_EVAL.md](MODULE_EVAL.md) | ~650 | ~15KB | 9 | Evaluator/runtime |
| [MODULE_MAIN.md](MODULE_MAIN.md) | ~600 | ~12KB | 5 | CLI/REPL |
| **Total** | **~7,200** | **~158KB** | **45+** | Comprehensive |

## 🎯 Quick Navigation

### I want to...

**Learn ParLang**
1. Start with [Main README](../README.md) for syntax basics
2. Follow [Examples Guide](EXAMPLES.md) for hands-on learning
3. Reference [Language Specification](LANGUAGE_SPEC.md) for details

**Use ParLang as a library**
1. Check [API Reference](API_REFERENCE.md)
2. See integration examples
3. Review module documentation as needed

**Understand the implementation**
1. Read [Architecture Guide](ARCHITECTURE.md) for overview
2. Dive into individual [Module Documentation](#module-documentation)
3. Study [Language Specification](LANGUAGE_SPEC.md) for semantics

**Contribute to ParLang**
1. Understand [Architecture Guide](ARCHITECTURE.md)
2. Read relevant [Module Documentation](#module-documentation)
3. Review extension points and design decisions
4. See testing strategies

**Implement a similar language**
1. Study [Language Specification](LANGUAGE_SPEC.md)
2. Analyze [Architecture Guide](ARCHITECTURE.md)
3. Review [Parser Module](MODULE_PARSER.md) for parser combinators
4. Study [Evaluator Module](MODULE_EVAL.md) for semantics

## 🔍 Mermaid Diagram Index

The documentation includes 45+ Mermaid diagrams visualizing:

**Architecture & System Design** (ARCHITECTURE.md)
- System architecture diagram
- Component interaction sequence diagram
- Data flow diagrams
- Module dependency graph
- Testing strategy diagram

**Language Specification** (LANGUAGE_SPEC.md)
- Syntax railroad diagrams
- Type hierarchy diagrams
- Scoping visualizations
- Evaluation strategy flowcharts
- Operator precedence diagrams

**Module Implementation**
- Parser pipeline and precedence (MODULE_PARSER.md)
- AST structure and trees (MODULE_AST.md)
- Evaluation flows and closures (MODULE_EVAL.md)
- CLI and REPL flows (MODULE_MAIN.md)

## 📝 Contributing to Documentation

When contributing to or extending the documentation:

1. **Maintain consistency** - Follow existing structure and style
2. **Use Mermaid** - Add diagrams for complex concepts
3. **Include examples** - Every concept should have code examples
4. **Cross-reference** - Link to related documentation
5. **Test examples** - Verify all code examples work
6. **Update index** - Keep this README.md in sync

## 🔗 External Resources

- [ParLang Repository](https://github.com/vansweej/parlang)
- [Combine Parser Combinators](https://github.com/Marwes/combine)
- [Rust Documentation](https://doc.rust-lang.org/)

## 📄 License

All documentation is part of the ParLang project and is licensed under the MIT License.

---

**Last Updated**: 2026-02-13  
**Documentation Version**: 1.0  
**ParLang Version**: 0.1.0

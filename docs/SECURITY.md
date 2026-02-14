# Security and Performance Best Practices

This document outlines security considerations and performance characteristics of ParLang.

## Table of Contents

- [Security Considerations](#security-considerations)
- [Performance Characteristics](#performance-characteristics)
- [Best Practices for Production Use](#best-practices-for-production-use)

## Security Considerations

### File System Access

#### Load Expression Security

The `load` expression allows loading and executing code from files:

```parlang
load "examples/stdlib.par" in double 21
```

**Security Considerations:**

1. **No Path Sandboxing**: The `load` expression can access any file the process has permission to read
   - Can load files outside the intended directory
   - Can potentially access sensitive configuration files
   - No built-in path whitelist or blacklist

2. **Path Traversal**: Relative paths like `../../../etc/passwd` are not blocked

3. **Symlink Following**: Symlinks are followed without validation

**Recommendations:**

For production use or when executing untrusted ParLang code:

1. **Run in a Sandbox**: Use OS-level sandboxing (containers, VMs, or chroot)
   ```bash
   # Example: Docker container with read-only filesystem
   docker run --rm -v $(pwd):/workspace:ro parlang:latest /workspace/script.par
   ```

2. **Set Working Directory**: Start ParLang in a restricted directory
   ```bash
   cd /safe/directory
   parlang script.par
   ```

3. **File System Permissions**: Use OS permissions to restrict access
   ```bash
   # Run as unprivileged user
   sudo -u parlang-user parlang script.par
   ```

4. **Validate Paths**: If embedding ParLang, validate paths before execution
   ```rust
   fn safe_load(path: &str) -> Result<String, Error> {
       let canonical = std::fs::canonicalize(path)?;
       let allowed_dir = std::fs::canonicalize("/safe/directory")?;
       
       if !canonical.starts_with(&allowed_dir) {
           return Err(Error::UnauthorizedPath);
       }
       
       std::fs::read_to_string(canonical)
   }
   ```

### Integer Overflow

**Mitigated in v0.2.0+**

ParLang now uses checked arithmetic operations:

```parlang
> 9223372036854775807 + 1
Error: Integer overflow in addition
```

**Before v0.2.0**: Integer operations could silently wrap around, potentially causing:
- Incorrect calculations
- Logic errors in conditionals
- Security issues in size calculations

**Current Status**: All arithmetic operations use `checked_add`, `checked_sub`, `checked_mul`, `checked_div`.

### Resource Exhaustion

#### Stack Overflow

**Risk**: Deeply nested expressions or non-tail-recursive functions can exhaust the stack:

```parlang
# This could cause stack overflow for large n
rec factorial -> fun n ->
    if n == 0 then 1 else n * factorial (n - 1)
    
factorial 100000  # May overflow stack
```

**Mitigation**:
- Use tail-recursive patterns when possible
- ParLang implements TCO (Tail Call Optimization) for tail-recursive functions
- Set stack size limits via OS: `ulimit -s 8192` (8MB stack)

```parlang
# Tail-recursive version (optimized by TCO)
let factorial = rec helper -> fun acc -> fun n ->
    if n == 0 then acc else helper (acc * n) (n - 1)
in factorial 1 100000  # Safe with TCO
```

#### Infinite Loops

**Risk**: Recursive functions without proper termination:

```parlang
rec loop -> fun x -> loop x
```

**Mitigation**:
- Implement timeout mechanisms in embedding applications
- Use OS-level resource limits: `timeout 10s parlang script.par`

#### Memory Exhaustion

**Risk**: Large data structures can consume excessive memory:

```parlang
# Creating a large list
type List a = Nil | Cons a (List a) in
let rec range -> fun n ->
    if n == 0 then Nil else Cons n (range (n - 1))
in range 10000000  # May exhaust memory
```

**Mitigation**:
- Use OS-level memory limits: `ulimit -v 1048576` (1GB virtual memory)
- Monitor memory usage in production
- Consider implementing lazy evaluation for large structures (future feature)

### Type System Security

ParLang's type system provides strong guarantees:

1. **Type Safety**: No type confusion at runtime
   - Can't accidentally interpret an integer as a boolean
   - Can't call non-functions
   - Records can't access non-existent fields

2. **Memory Safety**: Rust's memory safety applies to the interpreter
   - No buffer overflows in the interpreter itself
   - No use-after-free bugs
   - No data races (single-threaded execution)

**Note**: Type checking is optional. For maximum safety, always enable type checking:
```bash
PARLANG_TYPECHECK=1 parlang script.par
```

### Denial of Service Vectors

#### Parser Complexity

Very large or deeply nested expressions can cause parsing delays:

```parlang
# Deeply nested parentheses
(((((((((((((((((...))))))))))))))))))

# Very long expressions
1 + 1 + 1 + 1 + ... (millions of terms)
```

**Mitigation**:
- Set timeouts for parsing: `timeout 5s parlang script.par`
- Limit input file size
- Consider implementing depth limits in the parser (future improvement)

#### Type Inference Complexity

Complex type constraints can slow down type inference:

```parlang
# Deeply nested function types
let f = fun a -> fun b -> fun c -> fun d -> fun e -> fun f -> ...
```

**Mitigation**:
- Limit expression complexity
- Type inference is generally O(n × m) where n is expression size, m is average type size
- For typical programs, this is not a concern

## Performance Characteristics

### Time Complexity

#### Parsing
- **Input size**: O(n) where n is input length
- **Nested structures**: Additional O(d) where d is nesting depth
- **Backtracking**: `attempt()` can cause re-parsing, potentially O(n²) in worst case

#### Type Inference
- **Expression traversal**: O(n) where n is number of AST nodes
- **Unification**: O(m) per operation where m is type size
- **Overall**: O(n × m) for typical programs
- **Worst case**: Can be exponential for pathological cases with many type variables

#### Evaluation
- **Expression evaluation**: O(n) where n is number of operations
- **Environment lookup**: O(1) average case (HashMap)
- **Pattern matching**: O(p × m) where p is number of patterns, m is pattern complexity
- **Tail recursion**: O(1) additional overhead (optimized with TCO)

### Space Complexity

#### Memory Usage

1. **AST Storage**: O(n) where n is expression size
   - Each node is a Rust enum (typically 48-80 bytes)
   - Nested expressions use Box pointers (8 bytes per box)

2. **Environment**: O(v) where v is number of variables in scope
   - Each binding stores variable name and value
   - HashMap overhead: ~32 bytes per entry + key/value size

3. **Type Information**: O(t) where t is number of type variables
   - Type representations: 24-48 bytes each
   - Substitution maps: HashMap overhead

4. **Call Stack**: O(d) where d is recursion depth
   - Each function call adds stack frame
   - TCO prevents stack growth for tail calls

#### Memory Optimization Strategies

**Current Implementation:**
- Uses `Box<T>` for recursive structures (prevents infinite size)
- Uses `HashMap` for efficient lookups
- Clones environments on scope extension (simple but memory-intensive)

**Potential Optimizations:**

1. **Persistent Data Structures**: Use `im-rs` crate
   ```rust
   use im::HashMap;  // Structural sharing reduces cloning overhead
   ```

2. **Reference Counting**: Use `Rc<T>` for shared data
   ```rust
   use std::rc::Rc;
   
   struct Environment {
       bindings: Rc<HashMap<String, Value>>,
       parent: Option<Rc<Environment>>,
   }
   ```

3. **Arena Allocation**: Pool allocate AST nodes
   ```rust
   use typed_arena::Arena;
   
   let arena = Arena::new();
   let expr = arena.alloc(Expr::Int(42));
   ```

### Benchmarking

To benchmark ParLang programs:

```bash
# Using hyperfine
hyperfine --warmup 3 'parlang script.par'

# Using time
/usr/bin/time -v parlang script.par

# Using cargo bench (for internal functions)
cargo bench
```

**Example Performance Numbers** (on typical hardware):

| Operation | Time | Notes |
|-----------|------|-------|
| Parse simple expression | ~10μs | `42 + 10` |
| Parse complex expression | ~100μs | Nested let/if/match |
| Type inference simple | ~50μs | Integer arithmetic |
| Type inference complex | ~500μs | Polymorphic functions |
| Evaluate factorial(20) | ~5μs | With TCO |
| Load small library | ~200μs | Parsing + evaluation |

### Performance Best Practices

#### 1. Use Tail Recursion

**Slow (non-tail-recursive)**:
```parlang
rec sum -> fun list ->
    match list with
    | Nil -> 0
    | Cons head tail -> head + sum tail  # Not tail position
```

**Fast (tail-recursive with TCO)**:
```parlang
let sum = rec helper -> fun acc -> fun list ->
    match list with
    | Nil -> acc
    | Cons head tail -> helper (acc + head) tail  # Tail position
in sum 0
```

#### 2. Minimize Environment Cloning

**Slow (multiple nested scopes)**:
```parlang
let x = 1 in
let y = 2 in
let z = 3 in
let a = 4 in
let b = 5 in
...  # Each let clones the environment
```

**Better (sequential let bindings)**:
```parlang
let x = 1;
let y = 2;
let z = 3;
...  # Uses same environment
```

#### 3. Type Checking Overhead

Type checking adds overhead. If performance is critical and code is trusted:

```bash
# With type checking: slower, safer
PARLANG_TYPECHECK=1 parlang script.par

# Without type checking: faster, less safe
parlang script.par
```

For production: enable type checking during development, optionally disable for deployment if performance-critical.

#### 4. Library Loading

Loading libraries has parsing overhead. Structure code to minimize loads:

**Slow**:
```parlang
load "lib.par" in
let x = f 1 in
load "lib.par" in
let y = f 2 in
...
```

**Fast**:
```parlang
load "lib.par" in
let x = f 1 in
let y = f 2 in
...
```

## Best Practices for Production Use

### 1. Security Checklist

- [ ] Run in sandboxed environment (container, VM, or restricted user)
- [ ] Validate all input file paths before loading
- [ ] Enable type checking for additional safety
- [ ] Set resource limits (memory, CPU, file descriptors)
- [ ] Use timeouts for script execution
- [ ] Regular security updates of dependencies
- [ ] Audit loaded libraries for malicious code

### 2. Error Handling

```rust
// Embedding ParLang safely
use std::time::Duration;
use std::process::Command;

fn run_parlang_safely(script: &str, timeout: Duration) -> Result<String, Error> {
    let output = Command::new("timeout")
        .arg(format!("{}s", timeout.as_secs()))
        .arg("parlang")
        .arg(script)
        .env("PARLANG_TYPECHECK", "1")
        .output()?;
    
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Err(Error::ExecutionFailed(String::from_utf8(output.stderr)?))
    }
}
```

### 3. Monitoring

Monitor these metrics in production:

- **Execution time**: Detect slow scripts or potential DoS
- **Memory usage**: Prevent memory exhaustion
- **Error rate**: Track type errors, runtime errors
- **File access patterns**: Detect unusual load operations

### 4. Configuration

Recommended production configuration:

```bash
#!/bin/bash
# Production runner script

# Set resource limits
ulimit -s 8192      # 8MB stack
ulimit -v 1048576   # 1GB virtual memory
ulimit -t 30        # 30 second CPU time

# Enable type checking
export PARLANG_TYPECHECK=1

# Run in restricted directory
cd /safe/parlang/workspace

# Execute with timeout
timeout 10s parlang "$1"
```

### 5. Testing

Before production deployment:

```bash
# Run all tests
cargo test

# Run with sanitizers (detect memory issues)
RUSTFLAGS="-Z sanitizer=address" cargo test

# Fuzz testing (requires cargo-fuzz)
cargo fuzz run parser_fuzzer

# Performance testing
cargo bench
```

## Future Security Improvements

Planned enhancements for future versions:

1. **Path Whitelisting**: Built-in path validation for `load`
2. **Resource Limits**: Configurable depth/size limits for parsing
3. **Timeout Support**: Built-in timeout mechanism
4. **Sandboxing API**: Safe embedding API with configurable restrictions
5. **Capability-based Security**: Fine-grained permission system
6. **Audit Logging**: Track all file access and system operations

## Reporting Security Issues

If you discover a security vulnerability in ParLang:

1. **Do not** open a public issue
2. Email security concerns to the maintainers
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We aim to respond within 48 hours and will coordinate disclosure.

## References

- **Rust Security Guidelines**: https://anssi-fr.github.io/rust-guide/
- **OWASP Interpreter Security**: https://owasp.org/www-community/vulnerabilities/
- **Sandboxing Best Practices**: See your OS documentation (Docker, Firejail, etc.)

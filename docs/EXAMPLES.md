# ParLang Examples Guide

A comprehensive guide to programming in ParLang, from basic concepts to advanced patterns.

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Basic Syntax](#basic-syntax)
3. [Arithmetic Operations](#arithmetic-operations)
4. [Comparison Operations](#comparison-operations)
5. [Variables and Let Bindings](#variables-and-let-bindings)
6. [Conditional Expressions](#conditional-expressions)
7. [Pattern Matching](#pattern-matching)
8. [Functions](#functions)
9. [Function Application](#function-application)
10. [Closures](#closures)
11. [Currying and Partial Application](#currying-and-partial-application)
12. [Loading Libraries](#loading-libraries)
13. [Advanced Patterns](#advanced-patterns)
14. [Common Patterns and Idioms](#common-patterns-and-idioms)
15. [Real-World Use Cases](#real-world-use-cases)
16. [Best Practices](#best-practices)
17. [Common Mistakes to Avoid](#common-mistakes-to-avoid)
18. [Example Files](#example-files)
19. [Map (Key-Value Dictionaries)](#map-key-value-dictionaries)

---

## Getting Started

ParLang is a simple functional programming language with ML-alike syntax. All programs are expressions that evaluate to a value. There are no statements, loops, or mutable variables.

### Running Examples

To try these examples:

**REPL Mode (Interactive):**
```bash
cargo run
```

The REPL supports both single-line and multiline input:
- Type expressions and press Enter for single-line input
- **Auto-Submit**: When a line ends with a semicolon (`;`) after a `let` assignment, it's automatically submitted
- For other multiline expressions, continue typing on new lines (you'll see `... ` prompt)
- Press Enter on an empty line to evaluate the complete expression
- **Persistent Environment**: Function definitions and library loads using semicolon syntax persist across evaluations
- **Optional Trailing Expression**: You can omit the trailing `0` or `in 0` for convenience

**Example REPL session:**
```
> 42
42
> let x = 10
... in x * 2

20
> let double = fun x -> x + x;
0
> double 21
42
> load "examples/stdlib.par"
0
> max 10 20
20
```

**File Mode:**
```bash
cargo run -- examples/simple.par
```

---

## Basic Syntax

### Literals

ParLang supports two basic types of literals:

**Integers:**
```parlang
42
-10
0
999
```

**Expected Output:**
```
42
-10
0
999
```

**Booleans:**
```parlang
true
false
```

**Expected Output:**
```
true
false
```

---

## Arithmetic Operations

ParLang supports standard arithmetic operations with familiar precedence rules.

### Addition

```parlang
1 + 2
```
**Output:** `3`

```parlang
10 + 32
```
**Output:** `42`

### Subtraction

```parlang
10 - 3
```
**Output:** `7`

```parlang
50 - 8
```
**Output:** `42`

### Multiplication

```parlang
6 * 7
```
**Output:** `42`

```parlang
2 * 21
```
**Output:** `42`

### Division

```parlang
84 / 2
```
**Output:** `42`

```parlang
100 / 5
```
**Output:** `20`

**Note:** Division by zero produces a runtime error:
```parlang
10 / 0
```
**Output:** `Error: Division by zero`

### Operator Precedence

Multiplication and division have higher precedence than addition and subtraction:

```parlang
1 + 2 * 3
```
**Output:** `7` (evaluated as `1 + (2 * 3)`)

```parlang
10 - 6 / 2
```
**Output:** `7` (evaluated as `10 - (6 / 2)`)

### Parentheses for Grouping

Use parentheses to override default precedence:

```parlang
(1 + 2) * 3
```
**Output:** `9`

```parlang
((5 + 3) * 2) / 4
```
**Output:** `4`

### Complex Arithmetic

```parlang
1 + 2 - 3 * 4 / 2
```
**Output:** `-5` (evaluated as `1 + 2 - ((3 * 4) / 2)` = `3 - 6`)

---

## Comparison Operations

Comparison operations evaluate to boolean values.

### Equality

```parlang
5 == 5
```
**Output:** `true`

```parlang
5 == 3
```
**Output:** `false`

```parlang
true == true
```
**Output:** `true`

### Inequality

```parlang
5 != 3
```
**Output:** `true`

```parlang
5 != 5
```
**Output:** `false`

### Less Than

```parlang
3 < 5
```
**Output:** `true`

```parlang
5 < 3
```
**Output:** `false`

### Less Than or Equal

```parlang
3 <= 5
```
**Output:** `true`

```parlang
5 <= 5
```
**Output:** `true`

### Greater Than

```parlang
5 > 3
```
**Output:** `true`

```parlang
3 > 5
```
**Output:** `false`

### Greater Than or Equal

```parlang
5 >= 3
```
**Output:** `true`

```parlang
5 >= 5
```
**Output:** `true`

### Comparison in Complex Expressions

```parlang
1 + 2 == 3
```
**Output:** `true` (addition is evaluated first)

```parlang
10 - 5 > 3
```
**Output:** `true` (evaluates as `(10 - 5) > 3` = `5 > 3`)

---

## Variables and Let Bindings

Variables in ParLang are immutable and created using `let` bindings.

### Basic Let Binding (Expression Form)

```parlang
let x = 42 in x
```
**Output:** `42`

**Explanation:**
1. Bind the value `42` to the name `x`
2. Evaluate the body expression `x`
3. Result is `42`

### Sequential Let Bindings (Program Form)

ParLang supports two syntaxes for let bindings:

1. **Traditional let-in syntax** - used within expressions
2. **Sequential let syntax with semicolons** - used for programs and top-level definitions

**Sequential syntax (recommended for multiple definitions):**

```parlang
let x = 10;
let y = 32;
x + y
```
**Output:** `42`

**Explanation:**
1. Bind `10` to `x`
2. Bind `32` to `y` (can reference `x`)
3. Evaluate `x + y`
4. Result is `42`

**Benefits of sequential syntax:**
- No nested `in` keywords required
- Cleaner, more readable code
- Better for defining multiple functions in libraries

### Let with Expressions

```parlang
let x = 10 in x + 32
```
**Output:** `42`

**Explanation:**
1. Bind `10` to `x`
2. Evaluate `x + 32` which becomes `10 + 32`
3. Result is `42`

### Nested Let Bindings (Traditional Syntax)

```parlang
let x = 1 in let y = 2 in x + y
```
**Output:** `3`

**Explanation:**
1. Bind `1` to `x`
2. In that scope, bind `2` to `y`
3. Evaluate `x + y` = `1 + 2`
4. Result is `3`

**Same with sequential syntax:**

```parlang
let x = 1;
let y = 2;
x + y
```
**Output:** `3`

### Variable Shadowing

Inner bindings can shadow outer ones:

```parlang
let x = 1 in let x = 2 in x
```
**Output:** `2`

**Explanation:** The inner `x` shadows the outer `x`.

```parlang
let x = 10 in let x = x + 1 in x
```
**Output:** `11`

**Explanation:** The inner `x` is bound to the value of the outer `x` plus 1.

**With sequential syntax:**

```parlang
let x = 10;
let x = x + 1;
x
```
**Output:** `11`

### Complex Let Expressions

```parlang
let a = 5 in let b = 10 in let c = a + b in c * 2
```
**Output:** `30`

**With sequential syntax:**

```parlang
let a = 5;
let b = 10;
let c = a + b;
c * 2
```
**Output:** `30`

---

## Conditional Expressions

If-then-else expressions allow conditional evaluation.

### Basic Conditionals

```parlang
if true then 1 else 2
```
**Output:** `1`

```parlang
if false then 1 else 2
```
**Output:** `2`

### Conditionals with Comparisons

```parlang
if 5 > 3 then 100 else 0
```
**Output:** `100`

```parlang
if 5 == 5 then 42 else 0
```
**Output:** `42`

### Conditionals in Let Bindings

```parlang
let x = 10 in if x > 5 then x * 2 else x
```
**Output:** `20`

### Nested Conditionals

```parlang
if true then if false then 1 else 2 else 3
```
**Output:** `2`

**Explanation:**
1. Outer condition is `true`, so evaluate then-branch
2. Then-branch is `if false then 1 else 2`
3. Inner condition is `false`, so evaluate else-branch
4. Result is `2`

### Complex Conditional Logic

```parlang
let x = 15
in if x < 10
   then 0
   else if x < 20
        then 1
        else 2
```
**Output:** `1`

**Note:** See the [conditional.par](../examples/conditional.par) example file for more patterns.

---

## Pattern Matching

Pattern matching provides a cleaner alternative to nested if-then-else chains. It evaluates patterns from top to bottom and executes the first matching arm.

### Basic Pattern Matching

```parlang
match 0 with
| 0 -> 1
| n -> n
```
**Output:** `1`

**Explanation:**
1. The scrutinee expression `0` is evaluated
2. Pattern `0` (literal) matches the value
3. Result expression `1` is evaluated and returned

### Pattern Matching with Variables

```parlang
match 42 with
| 0 -> 1
| n -> n
```
**Output:** `42`

**Explanation:**
1. The scrutinee expression `42` is evaluated
2. Pattern `0` doesn't match
3. Pattern `n` (variable) matches and binds `n = 42`
4. Result expression `n` evaluates to `42`

### Wildcard Pattern

```parlang
match 100 with
| 0 -> 1
| _ -> 999
```
**Output:** `999`

**Explanation:**
- The wildcard pattern `_` matches any value without binding it to a variable
- Useful as a catch-all pattern at the end of a match expression

### Pattern Matching with Booleans

```parlang
match true with
| true -> 1
| false -> 0
```
**Output:** `1`

### Multiple Arms

```parlang
match 2 with
| 0 -> 10
| 1 -> 20
| 2 -> 30
| _ -> 40
```
**Output:** `30`

**Explanation:** Patterns are tried in order. The first matching pattern determines the result.

### Pattern Matching in Functions

```parlang
let abs = fun n -> match n with
| 0 -> 0
| n -> if n < 0 then 0 - n else n
in abs (-5)
```
**Output:** `5`

### Factorial with Pattern Matching

Instead of nested if-then-else:
```parlang
let factorial = rec f -> fun n ->
    if n == 0
    then 1
    else n * f (n - 1)
in factorial 5
```

Use pattern matching:
```parlang
let factorial = rec fact -> fun n ->
    match n with
    | 0 -> 1
    | n -> n * fact (n - 1)
in factorial 5
```
**Output:** `120`

### Fibonacci with Pattern Matching

```parlang
let fib = rec f -> fun n ->
    match n with
    | 0 -> 0
    | 1 -> 1
    | n -> f (n - 1) + f (n - 2)
in fib 7
```
**Output:** `13`

### Replacing Nested If-Then-Else

**Before (nested conditionals):**
```parlang
let category = fun n ->
    if n == 0 then 0
    else if n == 1 then 1
    else if n == 2 then 2
    else 999
```

**After (pattern matching):**
```parlang
let category = fun n ->
    match n with
    | 0 -> 0
    | 1 -> 1
    | 2 -> 2
    | _ -> 999
```

**Benefits:**
- More readable and maintainable
- Clearer intent - explicitly matching values
- Easier to add or modify cases
- Less error-prone than nested if-then-else chains

**Note:** See the [pattern_matching.par](../examples/pattern_matching.par) example file for more patterns.

---

## Functions

Functions are first-class values in ParLang. They can be passed as arguments, returned from other functions, and stored in variables.

### Anonymous Functions

```parlang
fun x -> x + 1
```
**Output:** `<function x>`

**Explanation:** Creates a function that takes parameter `x` and returns `x + 1`.

### Function Syntax

```
fun parameter -> body
```

- `fun` is the keyword
- `parameter` is the variable name
- `->` separates parameter from body
- `body` is any expression that can use the parameter

### Identity Function

```parlang
fun x -> x
```
**Output:** `<function x>`

**Explanation:** Returns its argument unchanged.

### Functions with Operations

```parlang
fun x -> x * 2
```
Creates a doubling function.

```parlang
fun x -> x + x
```
Also creates a doubling function (equivalent to above).

### Multi-Parameter Functions

ParLang functions take only one parameter, but you can create multi-parameter functions using currying (see [Currying section](#currying-and-partial-application)):

```parlang
fun x -> fun y -> x + y
```

This creates a function that takes `x` and returns another function that takes `y`.

---

## Function Application

Function application is how you call functions with arguments.

### Basic Application

```parlang
(fun x -> x + 1) 41
```
**Output:** `42`

**Explanation:**
1. Create function `fun x -> x + 1`
2. Apply it to argument `41`
3. Result is `41 + 1` = `42`

### Application with Let

```parlang
let inc = fun x -> x + 1 in inc 41
```
**Output:** `42`

**Explanation:**
1. Bind the increment function to name `inc`
2. Apply `inc` to `41`
3. Result is `42`

### Multiple Applications

```parlang
let double = fun x -> x + x
in double (double 5)
```
**Output:** `20`

**Explanation:**
1. `double 5` evaluates to `10`
2. `double 10` evaluates to `20`

### Application Syntax

Application is left-associative and written without parentheses (unless needed for grouping):

```
function argument
```

Multiple applications:
```
f x y z
```
Evaluates as `((f x) y) z` (left-to-right).

---

## Closures

Functions can capture variables from their surrounding environment, creating closures.

### Basic Closure

```parlang
let x = 10 in fun y -> x + y
```
**Output:** `<function y>`

**Explanation:** The function captures the value of `x` (10) from the environment.

### Using a Closure

```parlang
let x = 10 in (fun y -> x + y) 32
```
**Output:** `42`

**Explanation:**
1. `x` is bound to `10`
2. Function `fun y -> x + y` captures `x`
3. Apply function to `32`
4. Evaluate `10 + 32` = `42`

### Returning Closures

```parlang
let makeAdder = fun x -> fun y -> x + y
in let add5 = makeAdder 5
in add5 10
```
**Output:** `15`

**Explanation:**
1. `makeAdder` is a function that returns a closure
2. `makeAdder 5` creates a closure that captures `5`
3. `add5` is that closure
4. `add5 10` evaluates to `5 + 10` = `15`

### Closure Capturing Multiple Variables

```parlang
let a = 10
in let b = 20
in let combine = fun x -> a + b + x
in combine 5
```
**Output:** `35`

**Explanation:** The function captures both `a` and `b`.

---

## Currying and Partial Application

Currying is the technique of transforming a multi-argument function into a sequence of single-argument functions.

### What is Currying?

Instead of a function that takes two arguments:
```
function(x, y) = x + y
```

We have a function that takes one argument and returns a function:
```
function(x) = (function(y) = x + y)
```

### Basic Currying Example

```parlang
let add = fun x -> fun y -> x + y
in add 5 10
```
**Output:** `15`

**Explanation:**
1. `add` is a curried function
2. `add 5` returns a function that adds 5 to its argument
3. `(add 5) 10` evaluates to `5 + 10` = `15`

### Partial Application

Partial application is applying a curried function to fewer arguments than it expects:

```parlang
let add = fun x -> fun y -> x + y
in let add5 = add 5
in add5 10
```
**Output:** `15`

**Explanation:**
1. `add 5` partially applies `add`, creating `add5`
2. `add5` is a function that adds 5 to its argument
3. `add5 10` = `15`

### Practical Example: Multiplication

```parlang
let multiply = fun x -> fun y -> x * y
in let double = multiply 2
in let triple = multiply 3
in double (triple 5)
```
**Output:** `30`

**Explanation:**
1. `triple 5` = `3 * 5` = `15`
2. `double 15` = `2 * 15` = `30`

### Three-Parameter Curried Function

```parlang
let add3 = fun x -> fun y -> fun z -> x + y + z
in add3 1 2 3
```
**Output:** `6`

```parlang
let add3 = fun x -> fun y -> fun z -> x + y + z
in let add1 = add3 1
in let add1and2 = add1 2
in add1and2 3
```
**Output:** `6`

**Note:** See the [currying.par](../examples/currying.par) example file for more patterns.

---

## Loading Libraries

ParLang supports loading functions from library files using the `load` expression. This enables code reuse and modular programming.

### Basic Library Loading

**Syntax:**
```parlang
load "path/to/library.par" in expression
```

**Example:**

Create a library file `mylib.par`:
```parlang
let double = fun x -> x * 2
in let triple = fun x -> x * 3
in 0
```

Use it in your program:
```parlang
load "mylib.par" in double 21
```

**Output:** `42`

### How Libraries Work

Library files can be structured using semicolon-separated let bindings. The final expression (often `0`) is ignored - only the bindings are extracted.

**Modern library structure (recommended):**
```parlang
let func1 = fun x -> ...;
let func2 = fun y -> ...;
let func3 = fun z -> ...;
0
```

**Traditional library structure (also supported):**
```parlang
let func1 = fun x -> ...
in let func2 = fun y -> ...
in let func3 = fun z -> ...
in 0
```

### Standard Library Example

ParLang includes a standard library with common functions:

**examples/stdlib.par:**
```parlang
let double = fun x -> x * 2;
let triple = fun x -> x * 3;
let quadruple = fun x -> double (double x);
let abs = fun x -> if x < 0 then 0 - x else x;
let max = fun a -> fun b -> if a > b then a else b;
let min = fun a -> fun b -> if a < b then a else b;
let compose = fun f -> fun g -> fun x -> f (g x);
let id = fun x -> x;
0
```

**Using stdlib:**
```parlang
load "examples/stdlib.par"
in let result = compose double triple 5
in result
```

**Output:** `30` (triple(5) = 15, then double(15) = 30)

### Multiple Functions from Library

You can use multiple functions from a loaded library:

```parlang
load "examples/stdlib.par"
in let a = double 10
in let b = triple 7
in a + b
```

**Output:** `41` (20 + 21)

### Nested Library Loading

Libraries can load other libraries:

**helpers.par:**
```parlang
let inc = fun x -> x + 1
in 0
```

**math.par:**
```parlang
load "helpers.par"
in let double_inc = fun x -> inc (inc x)
in 0
```

**main program:**
```parlang
load "math.par"
in double_inc 10
```

**Output:** `12`

### Math Library Example

**examples/math.par:**
```parlang
let square = fun x -> x * x
in let cube = fun x -> x * x * x
in let avg = fun a -> fun b -> (a + b) / 2
in 0
```

**Using math library:**
```parlang
load "examples/math.par"
in let result = avg (square 4) (square 6)
in result
```

**Output:** `26` (average of 16 and 36)

### Best Practices for Libraries

1. **Structure as nested lets:** Always use nested `let` expressions
2. **End with dummy value:** Typically use `0` as the final expression
3. **Document your functions:** Use clear names and organize logically
4. **Relative paths:** Load paths are relative to the current working directory
5. **No circular dependencies:** Avoid libraries that load each other

### Common Use Cases

**Utility functions library:**
```parlang
let identity = fun x -> x
in let constant = fun x -> fun y -> x
in let flip = fun f -> fun x -> fun y -> f y x
in 0
```

**Mathematical operations:**
```parlang
let factorial = fun n -> if n == 0 then 1 else n * factorial (n - 1)
in let power = fun base -> fun exp -> if exp == 0 then 1 else base * power base (exp - 1)
in 0
```

**Higher-order function utilities:**
```parlang
let compose = fun f -> fun g -> fun x -> f (g x)
in let pipe = fun f -> fun g -> fun x -> g (f x)
in let apply = fun f -> fun x -> f x
in 0
```

---

## Advanced Patterns

### Composition Pattern

Composing functions to create new functionality:

```parlang
let double = fun x -> x * 2
in let inc = fun x -> x + 1
in let doubleAndInc = fun x -> inc (double x)
in doubleAndInc 5
```
**Output:** `11` (5 * 2 = 10, then 10 + 1 = 11)

### Quadruple Function

Building on simpler functions:

```parlang
let double = fun x -> x + x
in let quadruple = fun x -> double (double x)
in quadruple 5
```
**Output:** `20`

**Explanation:**
1. `double 5` = `10`
2. `double 10` = `20`

**Note:** See the [simple.par](../examples/simple.par) example file.

### Conditional Function Selection

```parlang
let abs = fun x ->
  if x < 0
  then 0 - x
  else x
in abs (-42)
```
**Output:** `42`

### Boolean Logic with Functions

Implementing logical operations:

```parlang
let not = fun b -> if b then false else true
in not true
```
**Output:** `false`

```parlang
let and = fun a -> fun b -> if a then b else false
in and true false
```
**Output:** `false`

### Maximum of Two Numbers

```parlang
let max = fun a -> fun b ->
  if a > b then a else b
in max 10 20
```
**Output:** `20`

### Minimum of Two Numbers

```parlang
let min = fun a -> fun b ->
  if a < b then a else b
in min 10 20
```
**Output:** `10`

### Clamping a Value

```parlang
let clamp = fun min_val -> fun max_val -> fun x ->
  if x < min_val
  then min_val
  else if x > max_val
       then max_val
       else x
in clamp 0 10 15
```
**Output:** `10` (15 is clamped to the maximum of 10)

### Higher-Order Function Pattern

A function that applies another function twice:

```parlang
let twice = fun f -> fun x -> f (f x)
in let inc = fun x -> x + 1
in twice inc 40
```
**Output:** `42` (increment 40 twice: 40 → 41 → 42)

---

## Common Patterns and Idioms

### 1. Builder Pattern

Creating specialized functions from general ones:

```parlang
let makeMultiplier = fun n -> fun x -> x * n
in let double = makeMultiplier 2
in let triple = makeMultiplier 3
in double 21
```
**Output:** `42`

### 2. Combinator Pattern

Functions that combine other functions:

```parlang
let compose = fun f -> fun g -> fun x -> f (g x)
in let inc = fun x -> x + 1
in let double = fun x -> x * 2
in let doubleAndInc = compose inc double
in doubleAndInc 20
```
**Output:** `41` (20 * 2 = 40, then 40 + 1 = 41)

### 3. Constant Functions

Functions that ignore their argument and return a constant:

```parlang
let constFortyTwo = fun x -> 42
in constFortyTwo 999
```
**Output:** `42`

### 4. Flip Pattern

Swapping parameter order:

```parlang
let subtract = fun x -> fun y -> x - y
in let flip = fun f -> fun a -> fun b -> f b a
in let reverseSubtract = flip subtract
in reverseSubtract 5 10
```
**Output:** `5` (10 - 5, parameters flipped)

### 5. Conditional Chain Pattern

```parlang
let sign = fun x ->
  if x < 0
  then 0 - 1
  else if x > 0
       then 1
       else 0
in sign 42
```
**Output:** `1`

### 6. Pipeline Pattern

Simulating a data pipeline:

```parlang
let x = 5
in let step1 = x * 2
in let step2 = step1 + 10
in let step3 = step2 / 4
in step3
```
**Output:** `5` ((5 * 2 + 10) / 4 = 20 / 4 = 5)

---

## Real-World Use Cases

### Configuration Values

```parlang
let debug = true
in let maxRetries = 3
in let timeout = 30
in if debug then maxRetries * 2 else maxRetries
```
**Output:** `6`

### Discount Calculator

```parlang
let applyDiscount = fun percent -> fun price ->
  price - (price * percent / 100)
in let tenPercentOff = applyDiscount 10
in let twentyPercentOff = applyDiscount 20
in tenPercentOff 100
```
**Output:** `90`

### Temperature Conversion

```parlang
let celsiusToFahrenheit = fun c -> (c * 9 / 5) + 32
in celsiusToFahrenheit 0
```
**Output:** `32`

```parlang
let fahrenheitToCelsius = fun f -> (f - 32) * 5 / 9
in fahrenheitToCelsius 32
```
**Output:** `0`

### Range Validation

```parlang
let inRange = fun min_val -> fun max_val -> fun value ->
  if value < min_val
  then false
  else if value > max_val
       then false
       else true
in inRange 0 100 50
```
**Output:** `true`

### Score Grading

```parlang
let grade = fun score ->
  if score >= 90
  then 1
  else if score >= 80
       then 2
       else if score >= 70
            then 3
            else 0
in grade 85
```
**Output:** `2` (using numbers to represent grades)

### Tax Calculation

```parlang
let calculateTax = fun rate -> fun amount ->
  amount * rate / 100
in let salesTax = calculateTax 8
in let incomeTax = calculateTax 25
in salesTax 100
```
**Output:** `8`

---

## Best Practices

### 1. Use Descriptive Names

**Good:**
```parlang
let calculateTotal = fun price -> fun quantity -> price * quantity
in calculateTotal 10 5
```

**Avoid:**
```parlang
let f = fun x -> fun y -> x * y
in f 10 5
```

### 2. Break Down Complex Expressions

**Good:**
```parlang
let x = 10
in let doubled = x * 2
in let withBonus = doubled + 5
in withBonus
```

**Avoid:**
```parlang
((10 * 2) + 5)
```

### 3. Use Let Bindings for Intermediate Results

**Good:**
```parlang
let basePrice = 100
in let withTax = basePrice + (basePrice * 8 / 100)
in let withShipping = withTax + 10
in withShipping
```

### 4. Favor Currying for Reusable Functions

**Good:**
```parlang
let multiply = fun x -> fun y -> x * y
in let double = multiply 2
in let triple = multiply 3
in double 5
```

### 5. Use Parentheses for Clarity

When nesting function applications or complex arithmetic:

**Good:**
```parlang
let result = (double (triple 5))
in result
```

### 6. Keep Functions Small and Focused

Each function should do one thing well:

**Good:**
```parlang
let double = fun x -> x * 2
in let addTen = fun x -> x + 10
in let process = fun x -> addTen (double x)
in process 5
```

### 7. Use Consistent Formatting

**Good:**
```parlang
let add = fun x -> fun y -> x + y
in let multiply = fun x -> fun y -> x * y
in add 5 (multiply 2 3)
```

---

## Common Mistakes to Avoid

### 1. Forgetting the "in" Keyword

**Wrong:**
```parlang
let x = 42 x
```
**Error:** Parse error

**Correct:**
```parlang
let x = 42 in x
```

### 2. Missing "else" Branch

**Wrong:**
```parlang
if x > 5 then 10
```
**Error:** Parse error

**Correct:**
```parlang
if x > 5 then 10 else 0
```

### 3. Using Keywords as Variable Names

**Wrong:**
```parlang
let if = 42 in if
```
**Error:** Parse error

**Correct:**
```parlang
let value = 42 in value
```

### 4. Type Mismatches

**Wrong:**
```parlang
if 42 then 1 else 2
```
**Error:** Type error: If condition must be a boolean

**Correct:**
```parlang
if true then 1 else 2
```

### 5. Applying Non-Functions

**Wrong:**
```parlang
42 100
```
**Error:** Type error: Application requires a function

**Correct:**
```parlang
let f = fun x -> x in f 42
```

### 6. Division by Zero

**Wrong:**
```parlang
10 / 0
```
**Error:** Division by zero

**Correct:**
```parlang
let safeDivide = fun x -> fun y ->
  if y == 0 then 0 else x / y
in safeDivide 10 0
```

### 7. Unbound Variables

**Wrong:**
```parlang
x + 10
```
**Error:** Unbound variable: x

**Correct:**
```parlang
let x = 5 in x + 10
```

### 8. Incorrect Function Application Order

**Wrong:**
```parlang
let subtract = fun x -> fun y -> x - y
in subtract 5 10
```
**Output:** `-5` (This is 5 - 10, which might not be what you wanted)

**If you wanted 10 - 5:**
```parlang
let subtract = fun x -> fun y -> x - y
in subtract 10 5
```
**Output:** `5`

### 9. Forgetting Parentheses in Nested Applications

**Wrong:**
```parlang
double double 5
```
This tries to apply `double` to `double` and then to `5`.

**Correct:**
```parlang
double (double 5)
```

### 10. Shadowing Without Intent

**Confusing:**
```parlang
let x = 10
in let x = 20
in x
```
**Output:** `20` (The inner x shadows the outer x)

**Clearer:**
```parlang
let x = 10
in let y = 20
in x + y
```

---

## Recursion

ParLang supports recursive functions using the `rec` keyword. Recursive functions can reference themselves by name, enabling iterative algorithms and elegant solutions to problems that naturally involve repetition.

### Basic Recursion

**Factorial Function:**
```parlang
rec factorial -> fun n ->
    if n == 0
    then 1
    else n * factorial (n - 1)
```

**Usage:**
```parlang
(rec factorial -> fun n ->
    if n == 0
    then 1
    else n * factorial (n - 1)
) 5
```

**Result:** `120`

**Explanation:**
- `rec factorial` creates a recursive function named `factorial`
- The function can call itself using the name `factorial`
- Base case: when `n == 0`, return `1`
- Recursive case: multiply `n` by `factorial (n - 1)`

### Named Recursive Functions with Let

You can bind recursive functions to names for reuse:

```parlang
let factorial = rec f -> fun n ->
    if n == 0
    then 1
    else n * f (n - 1)
in factorial 10
```

**Result:** `3628800`

### Fibonacci Sequence

```parlang
let fib = rec fibonacci -> fun n ->
    if n == 0
    then 0
    else if n == 1
    then 1
    else fibonacci (n - 1) + fibonacci (n - 2)
in fib 10
```

**Result:** `55`

**Explanation:**
- Base cases: `fib(0) = 0`, `fib(1) = 1`
- Recursive case: sum of previous two Fibonacci numbers

### Tail Recursion with Accumulator

**Sum from 1 to N (tail recursive):**
```parlang
let sum_to_n = rec helper -> fun acc -> fun n ->
    if n == 0
    then acc
    else helper (acc + n) (n - 1)
in sum_to_n 0 100
```

**Result:** `5050`

**Explanation:**
- Uses an accumulator (`acc`) to maintain running sum
- Tail recursive: the recursive call is the last operation
- Tail call optimization prevents stack overflow for large N

### Greatest Common Divisor (GCD)

```parlang
let gcd = rec gcd_helper -> fun a -> fun b ->
    if b == 0
    then a
    else gcd_helper b (a - (a / b) * b)
in gcd 48 18
```

**Result:** `6`

**Explanation:**
- Implements Euclidean algorithm
- Tail recursive for efficiency
- Uses modulo operation via division and multiplication

### Checking Even Numbers

```parlang
let is_even = rec check -> fun n ->
    if n == 0
    then true
    else if n == 1
    then false
    else check (n - 2)
in is_even 42
```

**Result:** `true`

**Explanation:**
- Recursively subtracts 2 until reaching base case
- Base cases: 0 is even, 1 is odd

### Power of 2

```parlang
let power_of_2 = rec pow -> fun n ->
    if n == 0
    then 1
    else 2 * pow (n - 1)
in power_of_2 8
```

**Result:** `256`

### Countdown to Zero

```parlang
let countdown = rec count -> fun n ->
    if n == 0
    then 0
    else count (n - 1)
in countdown 1000
```

**Result:** `0`

**Note:** Demonstrates that recursion can handle deep call stacks when tail-optimized.

### Recursive Functions in Libraries

Create a library file `recursion.par`:

```parlang
let factorial = rec f -> fun n ->
    if n == 0 then 1 else n * f (n - 1);
let fibonacci = rec fib -> fun n ->
    if n == 0 then 0
    else if n == 1 then 1
    else fib (n - 1) + fib (n - 2);
0
```

Use the library:

```parlang
load "examples/recursion.par"
in let f10 = factorial 10
in let fib10 = fibonacci 10
in fib10
```

**Result:** `55`

### Tail Call Optimization (TCO)

ParLang implements tail call optimization for recursive functions. When the recursive call is in tail position (the last operation before returning), the evaluator uses iteration instead of recursion, preventing stack overflow.

**Tail-recursive (optimized):**
```parlang
rec helper -> fun acc -> fun n ->
    if n == 0
    then acc
    else helper (acc + n) (n - 1)
```

The recursive call `helper (acc + n) (n - 1)` is in tail position, so TCO applies.

**Not tail-recursive:**
```parlang
rec sum -> fun n ->
    if n == 0
    then 0
    else n + sum (n - 1)
```

The addition `n + sum (n - 1)` happens *after* the recursive call returns, so this is not tail-recursive.

**Best Practice:** When possible, use accumulator parameters to make recursive functions tail-recursive for better performance with large inputs.

---

## Example Files

The `examples/` directory contains practical ParLang programs:

### simple.par

Demonstrates function composition and let bindings:

```parlang
let double = fun x -> x + x
in let quadruple = fun x -> double (double x)
in quadruple 5
```

**Output:** `20`

**Concepts demonstrated:**
- Function definition
- Let bindings
- Function composition
- Multiple function applications

**To run:**
```bash
cargo run -- examples/simple.par
```

### conditional.par

Shows conditional expressions:

```parlang
if 5 > 3
then 100
else 0
```

**Output:** `100`

**Concepts demonstrated:**
- If-then-else expressions
- Comparison operators
- Multi-line formatting

**To run:**
```bash
cargo run -- examples/conditional.par
```

### currying.par

Illustrates currying and partial application:

```parlang
let add = fun x -> fun y -> x + y
in let add5 = add 5
in add5 10
```

**Output:** `15`

**Concepts demonstrated:**
- Curried functions
- Partial application
- Returning functions from functions
- Creating specialized functions

**To run:**
```bash
cargo run -- examples/currying.par
```

### stdlib.par

A standard library with commonly used functions:

```parlang
let double = fun x -> x * 2
in let triple = fun x -> x * 3
in let quadruple = fun x -> double (double x)
in let abs = fun x -> if x < 0 then 0 - x else x
in let max = fun a -> fun b -> if a > b then a else b
in let min = fun a -> fun b -> if a < b then a else b
in let compose = fun f -> fun g -> fun x -> f (g x)
in let id = fun x -> x
in 0
```

**Concepts demonstrated:**
- Library file structure
- Multiple function definitions
- Higher-order functions (compose)
- Utility functions

**Usage:**
```parlang
load "examples/stdlib.par" in double 21
```

### math.par

Mathematical utility functions:

```parlang
let square = fun x -> x * x
in let cube = fun x -> x * x * x
in let avg = fun a -> fun b -> (a + b) / 2
in 0
```

**Concepts demonstrated:**
- Mathematical operations
- Library organization
- Reusable mathematical functions

**Usage:**
```parlang
load "examples/math.par" in square 7
```

### use_stdlib.par

Demonstrates loading and using library functions:

```parlang
load "examples/stdlib.par"
in let result = compose double triple 5
in result
```

**Output:** `30`

**Concepts demonstrated:**
- Loading external libraries
- Using library functions
- Function composition from libraries

**To run:**
```bash
cargo run -- examples/use_stdlib.par
```

---

### factorial.par

Demonstrates basic recursive factorial function:

```parlang
let factorial = rec f -> fun n ->
    if n == 0
    then 1
    else n * f (n - 1)
in factorial 10
```

**Output:** `3628800`

**Concepts demonstrated:**
- Recursive function definition
- Self-referencing functions
- Base case and recursive case

**To run:**
```bash
cargo run -- examples/factorial.par
```

---

### recursion.par

Library of common recursive functions:

```parlang
let factorial = rec f -> fun n ->
    if n == 0
    then 1
    else n * f (n - 1);

let fibonacci = rec fib -> fun n ->
    if n == 0
    then 0
    else if n == 1
    then 1
    else fib (n - 1) + fib (n - 2);

let sum_to_n = rec sum_helper -> fun acc -> fun n ->
    if n == 0
    then acc
    else sum_helper (acc + n) (n - 1);

let gcd = rec gcd_helper -> fun a -> fun b ->
    if b == 0
    then a
    else gcd_helper b (a - (a / b) * b);

0
```

**Concepts demonstrated:**
- Multiple recursive function definitions
- Tail recursion with accumulator
- Library structure for reusable recursive functions

**To run as library:** Use with `load "examples/recursion.par" in ...`

---

### use_recursion.par

Demonstrates loading and using recursive functions from a library:

```parlang
load "examples/recursion.par"
in let f5 = factorial 5
in let f10 = factorial 10
in let fib10 = fibonacci 10
in let sum100 = sum_to_n 0 100
in let gcd_result = gcd 48 18
in let pow8 = power_of_2 8
in pow8
```

**Output:** `256`

**Concepts demonstrated:**
- Loading recursive function libraries
- Reusing recursive functions
- Combining multiple recursive operations

**To run:**
```bash
cargo run -- examples/use_recursion.par
```

---

## Tuples

Tuples are first-class values that group multiple values together. They support heterogeneous types, projection, and pattern matching.

### Basic Tuple Creation

```parlang
# Simple tuple
(42, true)
```
**Output:** `(42, true)`

```parlang
# Empty tuple (unit type)
()
```
**Output:** `()`

```parlang
# Nested tuples
((1, 2), (3, 4))
```
**Output:** `((1, 2), (3, 4))`

### Tuple Projection

Access tuple elements using zero-based indexing:

```parlang
# Access first element
(10, 20).0
```
**Output:** `10`

```parlang
# Access second element
(10, 20).1
```
**Output:** `20`

```parlang
# Chained projection (left-associative)
((1, 2), (3, 4)).0.1
```
**Output:** `2`

### Tuples with Functions

```parlang
# Function returning a tuple
let make_pair = fun x -> (x, x + 1)
in make_pair 5
```
**Output:** `(5, 6)`

```parlang
# Swap function using tuples
let swap = fun p -> (p.1, p.0)
in swap (5, 10)
```
**Output:** `(10, 5)`

```parlang
# Tuple containing a function
let data = (42, fun x -> x * 2)
in data.1 21
```
**Output:** `42`

### Pattern Matching with Tuples

Destructure tuples in match expressions:

```parlang
# Basic tuple destructuring
match (10, 20) with
| (0, 0) -> 0
| (x, y) -> x + y
```
**Output:** `30`

```parlang
# Tuple pattern with literal
match (0, 5) with
| (0, y) -> y
| (x, y) -> x
```
**Output:** `5`

```parlang
# Tuple pattern with wildcard
match (10, 20) with
| (x, _) -> x
```
**Output:** `10`

```parlang
# Nested tuple pattern matching
match ((1, 2), 3) with
| ((a, b), c) -> a + b + c
```
**Output:** `6`

### Practical Examples

**Point operations:**

```parlang
let p1 = (0, 0) in
let p2 = (3, 4) in
let dx = p2.0 - p1.0 in
let dy = p2.1 - p1.1 in
dx * dx + dy * dy
```
**Output:** `25`

**Fibonacci with tuple state:**

```parlang
let fib = rec fib -> fun n ->
    if n == 0 then (0, 1)
    else if n == 1 then (1, 1)
    else
        let prev = fib (n - 1) in
        (prev.1, prev.0 + prev.1)
in (fib 6).0
```
**Output:** `8`

**Divmod using tuples and recursion:**

```parlang
let divmod = rec divmod -> fun p ->
    match p with
    | (n, d) ->
        if n < d then (0, n)
        else
            let result = divmod (n - d, d) in
            (result.0 + 1, result.1)
in divmod (17, 5)
```
**Output:** `(3, 2)`

---

## Map (Key-Value Dictionaries)

ParLang provides Map data structures as library implementations, demonstrating that complex data structures don't need to be language primitives. Maps store key-value pairs and support efficient lookup, insertion, and deletion.

### Association List Map (Simple)

The simplest map implementation using a list of tuples. Best for small maps (< 20 entries).

**Creating and using a simple map:**

```parlang
type List a = Nil | Cons a (List a) in
type Option a = Some a | None in
type Tuple k v = Tuple k v in
let empty = Nil in
let insert = fun key -> fun value -> fun map ->
  Cons (Tuple key value) map
in
let lookup = rec lookup -> fun key -> fun map ->
  match map with
  | Nil -> None
  | Cons pair rest ->
      (match pair with
      | Tuple k v ->
          if k == key then Some v
          else lookup key rest)
in
let m = insert 1 42 (insert 2 30 empty) in
lookup 1 m
```
**Output:** `Some(42)`

### Binary Search Tree Map (Efficient)

A more efficient map using a binary search tree. Keys are automatically sorted and lookups are faster.

**Creating and using a tree map:**

```parlang
type TreeMap k v = Empty | Node k v (TreeMap k v) (TreeMap k v) in
type Option a = Some a | None in
let empty = Empty in
let insert = rec insert -> fun key -> fun value -> fun map ->
  match map with
  | Empty -> Node key value Empty Empty
  | Node k v left right ->
      if key == k then Node key value left right
      else if key < k then Node k v (insert key value left) right
      else Node k v left (insert key value right)
in
let lookup = rec lookup -> fun key -> fun map ->
  match map with
  | Empty -> None
  | Node k v left right ->
      if key == k then Some v
      else if key < k then lookup key left
      else lookup key right
in
let m = insert 5 50 (insert 3 30 (insert 7 70 empty)) in
lookup 5 m
```
**Output:** `Some(50)`

### Practical Map Operations

**Counting frequencies:**

```parlang
type TreeMap k v = Empty | Node k v (TreeMap k v) (TreeMap k v) in
type List a = Nil | Cons a (List a) in
type Option a = Some a | None in
let empty = Empty in
let insert = rec insert -> fun key -> fun value -> fun map ->
  match map with
  | Empty -> Node key value Empty Empty
  | Node k v left right ->
      if key == k then Node key value left right
      else if key < k then Node k v (insert key value left) right
      else Node k v left (insert key value right)
in
let lookup = rec lookup -> fun key -> fun map ->
  match map with
  | Empty -> None
  | Node k v left right ->
      if key == k then Some v
      else if key < k then lookup key left
      else lookup key right
in
let count = rec count -> fun items -> fun map ->
  match items with
  | Nil -> map
  | Cons item rest ->
      let current = (match lookup item map with
      | Some n -> n
      | None -> 0) in
      count rest (insert item (current + 1) map)
in
let items = Cons 1 (Cons 2 (Cons 1 (Cons 3 (Cons 2 (Cons 1 Nil))))) in
let frequencies = count items empty in
lookup 1 frequencies
```
**Output:** `Some(3)`

**Map operations (transforming values):**

```parlang
type TreeMap k v = Empty | Node k v (TreeMap k v) (TreeMap k v) in
let empty = Empty in
let insert = rec insert -> fun key -> fun value -> fun map ->
  match map with
  | Empty -> Node key value Empty Empty
  | Node k v left right ->
      if key == k then Node key value left right
      else if key < k then Node k v (insert key value left) right
      else Node k v left (insert key value right)
in
let map_values = rec map_values -> fun f -> fun m ->
  match m with
  | Empty -> Empty
  | Node k v left right ->
      Node k (f v) (map_values f left) (map_values f right)
in
let fold = rec fold -> fun f -> fun acc -> fun m ->
  match m with
  | Empty -> acc
  | Node k v left right ->
      let acc1 = fold f acc left in
      let acc2 = f acc1 k v in
      fold f acc2 right
in
let m = insert 1 10 (insert 2 20 (insert 3 30 empty)) in
let doubled = map_values (fun x -> x * 2) m in
fold (fun acc -> fun k -> fun v -> acc + v) 0 doubled
```
**Output:** `120`

For more details on Map operations and implementations, see [MAP_LIBRARY.md](MAP_LIBRARY.md).

---

## Summary

ParLang is a minimalist functional language that teaches core functional programming concepts:

- **Immutability**: All bindings are immutable
- **First-class functions**: Functions are values
- **Closures**: Functions capture their environment
- **Recursion**: Named recursive functions with tail call optimization
- **Currying**: Multi-parameter functions via nested single-parameter functions
- **Expression-oriented**: Everything is an expression that evaluates to a value
- **Tuples**: Group multiple values together with projection and pattern matching
- **Pattern matching**: Destructure values in match expressions

These examples should give you a solid foundation for writing ParLang programs. Experiment with the patterns and build your own functions!

For more information, see:
- [Language Specification](LANGUAGE_SPEC.md)
- [Architecture Documentation](ARCHITECTURE.md)
- [API Reference](API_REFERENCE.md)

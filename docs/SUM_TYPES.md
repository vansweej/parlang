# Sum Types (Algebraic Data Types)

Sum types (also known as algebraic data types or tagged unions) allow you to define types with multiple variants, each potentially carrying different data. This is a powerful feature for type-safe data modeling.

## Defining Sum Types

Use the `type` keyword followed by type parameters (if any), then `=`, then a list of constructors separated by `|`:

```parlang
# Option type - represents an optional value
type Option a = Some a | None

# Either type - represents a value that can be one of two types
type Either a b = Left a | Right b

# List type (recursive) - represents a linked list
type List a = Nil | Cons a (List a)

# Tree type - represents a binary tree
type Tree a = Leaf | Node a (Tree a) (Tree a)
```

## Using Constructors

Constructors are used to create values of sum types. Constructor names must start with an uppercase letter:

```parlang
# Create values with constructors
type Option a = Some a | None in
let x = Some 42 in
let y = None in
x  # Result: Some(42)
```

Constructors with multiple arguments:

```parlang
type List a = Nil | Cons a (List a) in
let list = Cons 1 (Cons 2 (Cons 3 Nil)) in
list  # Result: Cons(1, Cons(2, Cons(3, Nil)))
```

## Pattern Matching

Use `match` expressions to destructure sum types and extract their values:

```parlang
type Option a = Some a | None in
let x = Some 42 in
match x with
| Some n -> n + 1
| None -> 0
# Result: 43
```

### Nested Patterns

Pattern matching works with nested constructors:

```parlang
type Option a = Some a | None in
let x = Some (Some 42) in
match x with
| Some (Some n) -> n
| Some None -> 0
| None -> 0
# Result: 42
```

### Wildcard Patterns

Use `_` to ignore values you don't need:

```parlang
type Option a = Some a | None in
let x = Some 99 in
match x with
| Some _ -> 1
| None -> 0
# Result: 1
```

## Polymorphic Types

Sum types can be parameterized with type variables (lowercase identifiers):

```parlang
# Option works with any type
type Option a = Some a | None in
let intOpt = Some 42 in
let boolOpt = Some true in
intOpt  # Works with integers
```

Type parameters allow you to write generic, reusable types:

```parlang
# Either with two type parameters
type Either a b = Left a | Right b in
let result = Left 10 in
result
```

## Recursive Types

Sum types can reference themselves, enabling recursive data structures:

```parlang
# Recursive list with length function
type List a = Nil | Cons a (List a) in
let rec length -> fun list ->
  match list with
  | Nil -> 0
  | Cons _ tail -> 1 + length tail
in
let myList = Cons 1 (Cons 2 (Cons 3 Nil)) in
length myList
# Result: 3
```

### Sum Function for Lists

```parlang
type List a = Nil | Cons a (List a) in
let rec sum -> fun list ->
  match list with
  | Nil -> 0
  | Cons head tail -> head + sum tail
in
let myList = Cons 1 (Cons 2 (Cons 3 Nil)) in
sum myList
# Result: 6
```

## Common Patterns

### Option for Nullable Values

Use `Option` to represent values that might be absent:

```parlang
type Option a = Some a | None in
let safeDivide = fun x -> fun y ->
  if y == 0 then None else Some (x / y)
in
match safeDivide 10 2 with
| Some result -> result
| None -> 0
# Result: 5
```

### Either for Error Handling

Use `Either` to represent computations that can succeed or fail:

```parlang
type Either a b = Left a | Right b in
let parseNumber = fun x ->
  if x > 0 then Right x else Left 0
in
match parseNumber 5 with
| Right n -> n * 2
| Left err -> err
# Result: 10
```

### Lists for Collections

```parlang
type List a = Nil | Cons a (List a) in
let rec map -> fun f -> fun list ->
  match list with
  | Nil -> Nil
  | Cons head tail -> Cons (f head) (map f tail)
in
let double = fun x -> x * 2 in
let list = Cons 1 (Cons 2 (Cons 3 Nil)) in
map double list
# Result: Cons(2, Cons(4, Cons(6, Nil)))
```

## Type Definition Scope

Type definitions introduce constructors that are available in the body expression:

```parlang
type Option a = Some a | None in
# Constructors Some and None are available here
Some 42
```

Multiple type definitions can be nested:

```parlang
type Option a = Some a | None in
type Either a b = Left a | Right b in
# Both sets of constructors are available here
let x = Some 10 in
let y = Left 20 in
x
```

## Constructor Arity

Constructors enforce the correct number of arguments:

```parlang
type Option a = Some a | None in
Some 1 2  # Error: Constructor Some expects 1 arguments, got 2
```

## Examples

### Result Type

A common pattern for operations that can fail:

```parlang
type Result a b = Ok a | Err b in
let divide = fun x -> fun y ->
  if y == 0
  then Err 0
  else Ok (x / y)
in
match divide 10 5 with
| Ok value -> value
| Err _ -> 0
# Result: 2
```

### Binary Tree

```parlang
type Tree a = Leaf | Node a (Tree a) (Tree a) in
let tree = Node 5
  (Node 3 Leaf Leaf)
  (Node 7 Leaf Leaf)
in
match tree with
| Leaf -> 0
| Node value _ _ -> value
# Result: 5
```

## Type Inference

The type checker provides basic support for sum types. Type definitions are transparent at the type level, allowing constructors to be used naturally within their scope.

## Limitations and Future Work

Current implementation:
- Basic type inference for sum types
- Constructor arity checking at runtime
- Pattern matching with exhaustiveness warnings (runtime errors for non-exhaustive matches)

Potential future enhancements:
- Compile-time exhaustiveness checking
- Better type error messages
- GADT support
- Deriving functions (Eq, Show, etc.)
- Record-style constructors

## See Also

- [Pattern Matching Guide](MATCH_EXPRESSIONS.md)
- [Type System Overview](TYPES.md)
- [Examples](../examples/)

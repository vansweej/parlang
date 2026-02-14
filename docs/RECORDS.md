# Record Types in ParLang

Records are **product types with named fields**, providing structured data with type-safe field access by name. They enable you to group related values together in an organized, self-documenting way.

## Table of Contents

- [Creating Records](#creating-records)
- [Field Access](#field-access)
- [Pattern Matching](#pattern-matching)
- [Functions with Records](#functions-with-records)
- [Type Inference](#type-inference)
- [Immutability](#immutability)
- [Advanced Patterns](#advanced-patterns)
- [Error Handling](#error-handling)
- [Design Decisions](#design-decisions)

## Creating Records

### Basic Record Construction

Records are created using curly brace syntax with field-value pairs:

```parlang
# Simple record with two fields
let person = { name: 42, age: 30 }

# Record with boolean fields
let config = { active: true, verbose: false }

# Empty record
let empty = {}
```

### Nested Records

Records can contain other records, enabling hierarchical data structures:

```parlang
# Nested address within person
let address = { street: 123, city: 456 }
let person = { name: 789, address: address }

# Deeply nested structures
let data = { a: { b: { c: 42 } } }
```

### Records from Expressions

Field values can be any expression:

```parlang
let x = 10
let y = 20
let point = { x: x + 5, y: y * 2 }  # { x: 15, y: 40 }
```

## Field Access

### Basic Field Access

Access record fields using dot notation:

```parlang
let person = { name: 42, age: 30 }
in person.age  # Returns 30
```

### Chained Field Access

Access nested fields by chaining dot notation:

```parlang
let person = { address: { city: 100 } }
in person.address.city  # Returns 100
```

### Field Access in Expressions

Use field access anywhere an expression is valid:

```parlang
let point = { x: 10, y: 20 }
in point.x + point.y  # Returns 30
```

## Pattern Matching

### Full Pattern Matching

Match all fields in a record:

```parlang
let person = { name: 42, age: 30 }
in match person with
| { name: n, age: a } -> n + a  # Returns 72
```

### Partial Pattern Matching

Match only specific fields - other fields are ignored:

```parlang
let person = { name: 42, age: 30, city: 100 }
in match person with
| { name: n } -> n  # Returns 42, ignores age and city
```

### Patterns with Wildcards

Use wildcards to ignore specific field values:

```parlang
let person = { name: 42, age: 30 }
in match person with
| { name: _, age: a } -> a  # Returns 30, ignores name value
```

### Patterns with Literals

Match against specific field values:

```parlang
let person = { status: 1, name: 42 }
in match person with
| { status: 1, name: n } -> n  # Matches only if status is 1
| _ -> 0
```

### Nested Pattern Matching

Patterns can match nested record structures:

```parlang
let data = { outer: { inner: 42 } }
in match data with
| { outer: { inner: n } } -> n  # Returns 42
```

## Functions with Records

### Records as Function Parameters

Pass records to functions:

```parlang
let getAge = fun person -> person.age

let person = { name: 42, age: 25 }
in getAge person  # Returns 25
```

### Records as Return Values

Return records from functions:

```parlang
let makePerson = fun name -> fun age -> 
  { name: name, age: age }

let person = makePerson 42 30  # Returns { name: 42, age: 30 }
```

### Curried Functions with Records

Build records incrementally with currying:

```parlang
let makePoint = fun x -> fun y -> { x: x, y: y }
let point = (makePoint 10) 20  # Returns { x: 10, y: 20 }
```

### Higher-Order Functions

Use records with higher-order functions:

```parlang
let mapField = fun f -> fun record -> 
  { value: f record.value }

let inc = fun x -> x + 1
let data = { value: 41 }
in (mapField inc data).value  # Returns 42
```

## Type Inference

ParLang automatically infers record types:

```parlang
# Type: { name: Int, age: Int }
let person = { name: 42, age: 30 }

# Type: Int
person.age

# Type: { age: t1 } -> t1
fun p -> p.age
```

### Polymorphic Record Types

Functions can work with any record having specific fields:

```parlang
# Works with any record that has an 'age' field
let getAge = fun p -> p.age

# Can be used with different record types
getAge { name: 42, age: 25 }        # Works
getAge { age: 30, active: true }    # Also works
```

### Type Checking

Record types are checked for field compatibility:

```parlang
# Type error: field 'age' not found
let makePerson = fun x -> { name: x }
let getPerson = fun p -> p.age
in getPerson (makePerson 42)  # Type error!
```

## Immutability

Records in ParLang are **immutable** - they cannot be modified after creation. To "update" a record, create a new one:

```parlang
# Create original record
let person = { name: 42, age: 30 }

# "Update" by creating new record
let olderPerson = { name: person.name, age: person.age + 1 }
in olderPerson  # { name: 42, age: 31 }
```

### Functional Updates

Build updated records using field access:

```parlang
let incrementAge = fun person ->
  { name: person.name, age: person.age + 1 }

let person = { name: 42, age: 30 }
let updated = incrementAge person  # { name: 42, age: 31 }
```

## Advanced Patterns

### Records with Function Fields

Store functions in records (method-like behavior):

```parlang
let obj = { 
  value: 42,
  getValue: fun x -> x 
}
in (obj.getValue) obj.value  # Returns 42
```

### Conditional Record Construction

Use if-expressions to create conditional records:

```parlang
let makeRecord = fun flag ->
  if flag 
  then { type: 1, data: 100 }
  else { type: 0, data: 0 }
```

### Record Transformation Pipelines

Chain functions to transform records:

```parlang
let addOne = fun r -> { value: r.value + 1 }
let double = fun r -> { value: r.value * 2 }

let initial = { value: 5 }
let result = double (addOne initial)  # { value: 12 }
```

## Error Handling

### Field Not Found

Accessing a non-existent field causes a runtime error:

```parlang
let person = { name: 42, age: 30 }
in person.salary  # Error: Field 'salary' not found
```

Error message includes available fields:
```
Field 'salary' not found. Available fields: ["age", "name"]
```

### Record Expected

Accessing a field on a non-record value:

```parlang
let x = 42
in x.field  # Error: Expected record, got Int(42)
```

### Type Errors

Type checker catches field mismatches:

```parlang
# Type error at compile time
let f = fun r -> r.age
let x = { name: 42 }
in f x  # Type error: record doesn't have 'age' field
```

## Design Decisions

### Why HashMap for Runtime?

Records use `HashMap<String, Value>` at runtime for **O(1) field access**, making field lookups efficient even for records with many fields.

### Why Vec for AST?

The AST uses `Vec<(String, Expr)>` to **preserve insertion order** for display purposes. This ensures that `{ name: 42, age: 30 }` displays in the order you wrote it.

### Structural Typing

Records use **structural typing** - two records with the same fields have the same type, regardless of how they were created. This enables:

- Flexible function parameters
- Natural polymorphism
- Easy composition

### Partial Pattern Matching

Records support **partial patterns** - you can match just the fields you care about. This is useful for:

- Extracting specific data
- Working with evolving data structures
- Building reusable pattern matchers

### No Field Update Syntax

ParLang records are **immutable by design**. There's no special syntax for updating fields because:

- Encourages functional programming style
- Simplifies reasoning about code
- Avoids mutation-related bugs

Future versions may add syntactic sugar like:
```parlang
# Hypothetical future syntax
{ person with age: 31 }
```

### Type Inference with Row Polymorphism

ParLang implements **row polymorphism** for record types, allowing functions to work with records that have **at least** certain fields, without requiring knowledge of all fields.

#### What is Row Polymorphism?

Row polymorphism enables flexible, reusable functions that work with any record containing specific fields:

```parlang
# This function works with ANY record that has an 'age' field
fun person -> person.age
# Type: { age: t0 | r1 } -> t0
```

The type `{ age: t0 | r1 }` means:
- The record must have an `age` field of type `t0`
- The `| r1` part (row variable) represents "any other fields"
- The function returns the type `t0` (the type of the age field)

#### Benefits of Row Polymorphism

**1. Flexible Functions**
```parlang
let getAge = fun r -> r.age
in let person = { name: 42, age: 30, city: 100 }
in let employee = { age: 25, department: 5 }
in getAge person + getAge employee
# Works! Returns 55
```

The same `getAge` function works with both `person` and `employee`, even though they have different fields.

**2. Type Safety**
```parlang
let getAge = fun r -> r.age
in let config = { port: 8080, active: true }
in getAge config
# Type error! 'age' field not found
```

Even though `getAge` is polymorphic, the type system still catches field access errors at compile time.

**3. Composable Functions**
```parlang
let getName = fun r -> r.name
let getAge = fun r -> r.age
let describeQualified only works with records that have both 'name' and 'age'
let describe = fun r -> getName r + getAge r
```

#### Row Variable Display

When type checking is enabled, you'll see row variables in function types:

```parlang
> fun p -> p.age
Type: { age: t0 | r0 } -> t0
```

- `t0` is a type variable (represents any type)
- `r0` is a row variable (represents "rest of the fields")
- The entire type means: "takes a record with at least an `age` field, returns that field's type"

#### Advanced Row Polymorphism Examples

**Multiple field access:**
```parlang
let addCoordinates = fun r -> r.x + r.y
in let point2D = { x: 10, y: 20 }
in let point3D = { x: 5, y: 15, z: 25 }
in addCoordinates point2D + addCoordinates point3D
# Returns 50
```

**Row polymorphism with currying:**
```parlang
let compareAges = fun r1 -> fun r2 -> r1.age == r2.age
in let person = { name: 42, age: 30 }
in let employee = { id: 123, age: 30, dept: 5 }
in compareAges person employee
# Returns true
```

**Row polymorphism with higher-order functions:**
```parlang
let mapAge = fun f -> fun r -> f r.age
in let double = fun x -> x + x
in let person = { name: 42, age: 21, active: true }
in mapAge double person
# Returns 42
```

#### Limitations

**Known Limitations of Current Implementation:**

1. **Multiple accesses on same row variable:** When accessing multiple fields on a function parameter in a single expression without a concrete record, the type system may not track all accesses properly:
   ```parlang
   # This works when applied to a concrete record:
   let addXY = fun r -> r.x + r.y
   in addXY { x: 10, y: 20 }  # OK!
   
   # But type inference for just the function may be limited
   fun r -> r.x + r.y  # Type checking may have issues
   ```

2. **Row unification complexity:** Complex row variable constraints (like ensuring two records share specific fields) may not be fully supported in all cases.

Despite these limitations, row polymorphism greatly enhances the flexibility and reusability of record-handling code while maintaining type safety.

## Examples

### Example 1: Point Operations

```parlang
let origin = { x: 0, y: 0 }
let point = { x: 3, y: 4 }

let distance = fun p1 -> fun p2 ->
  let dx = p1.x - p2.x in
  let dy = p1.y - p2.y in
  dx * dx + dy * dy

in distance origin point  # Returns 25
```

### Example 2: Configuration Management

```parlang
let defaultConfig = { port: 8080, verbose: false }
let customConfig = { port: 3000, verbose: true }

let selectConfig = fun useCustom ->
  if useCustom 
  then customConfig
  else defaultConfig

in (selectConfig true).port  # Returns 3000
```

### Example 3: Data Pipeline

```parlang
let data = { value: 10, count: 5 }

let process = fun r ->
  let doubled = { value: r.value * 2, count: r.count } in
  let incremented = { value: doubled.value, count: doubled.count + 1 } in
  incremented

in (process data).value  # Returns 20
```

### Example 4: Pattern-Based Dispatch

```parlang
let handleRequest = fun request ->
  match request with
  | { method: 0, path: p } -> p        # GET
  | { method: 1, data: d } -> d        # POST
  | _ -> 0                              # Other

let getRequest = { method: 0, path: 100 }
in handleRequest getRequest  # Returns 100
```

## Limitations and Future Work

### Current Limitations

1. **No record update syntax** - must manually copy all fields
2. **No field punning** - can't write `{ name, age }` instead of `{ name: name, age: age }`
3. **Simple row polymorphism** - doesn't track all possible field presence
4. **No record extension** - can't inherit or extend record types

### Planned Enhancements

Future versions may include:

- **Record update syntax**: `{ record with field: newValue }`
- **Field punning**: `{ name, age }` for `{ name: name, age: age }`
- **Type aliases for records**: `type Person = { name: Int, age: Int }`
- **Advanced row polymorphism**: More precise type tracking
- **Record concatenation**: Merge records together

## Summary

Records in ParLang provide:

✓ **Named fields** for clear, self-documenting code  
✓ **Type safety** with automatic type inference  
✓ **Immutability** for functional programming  
✓ **Pattern matching** for flexible data access  
✓ **Nested structures** for complex data  
✓ **Polymorphism** for reusable functions  

Records make ParLang suitable for building structured applications while maintaining the elegance of functional programming.

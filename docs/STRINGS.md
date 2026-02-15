# String Support in ParLang

ParLang provides comprehensive string support through syntactic sugar that desugars string literals to `List Char`.

## Overview

Strings in ParLang are not a primitive type but rather a convenient syntax for working with lists of characters. This design:
- Leverages the existing type system (List and Char)
- Maintains functional purity (strings are immutable)
- Allows all list operations to work on strings
- Provides type safety through the type system

## String Literals

### Basic Syntax

String literals are enclosed in double quotes:

```parlang
"hello"        # A string
"world"        # Another string
""             # Empty string
```

### Escape Sequences

Strings support common escape sequences:

| Escape | Meaning | Unicode |
|--------|---------|---------|
| `\n` | Newline | U+000A |
| `\t` | Tab | U+0009 |
| `\r` | Carriage return | U+000D |
| `\\` | Backslash | U+005C |
| `\"` | Double quote | U+0022 |
| `\'` | Single quote | U+0027 |
| `\0` | Null character | U+0000 |

**Examples:**
```parlang
"hello\nworld"          # Two lines
"tab\there"             # Tab character
"quote\"inside"         # Escaped quote
"backslash\\"           # Escaped backslash
```

### Unicode Support

ParLang strings support full Unicode:

```parlang
"Hello, 世界"           # Chinese
"Γειά σου κόσμε"        # Greek
"emoji: 🎉🎊"          # Emoji
```

## Type System Integration

### Type Definition

Strings have type `List Char`:

```parlang
> "hello"
Type: List Char
```

You can create a type alias for documentation:

```parlang
type String = List Char
```

### Desugaring

String literals desugar to nested `Cons` and `Nil` constructors:

```parlang
"abc"
# Becomes:
Cons('a', Cons('b', Cons('c', Nil)))

""
# Becomes:
Nil
```

## String Operations

### Standard Library

Load the string standard library:

```parlang
load "examples/string.par" in
```

### Available Functions

#### `strlen : List Char -> Int`
Returns the length of a string.

```parlang
strlen "hello"                 # 5
strlen ""                      # 0
strlen "🎉"                   # 1
```

#### `strcat : List Char -> List Char -> List Char`
Concatenates two strings.

```parlang
strcat "hello" " world"        # "hello world"
strcat "" "test"               # "test"
strcat "a" ""                  # "a"
```

#### `streq : List Char -> List Char -> Bool`
Tests string equality.

```parlang
streq "hello" "hello"          # true
streq "hello" "world"          # false
streq "" ""                    # true
```

#### `contains : List Char -> Char -> Bool`
Tests if a string contains a character.

```parlang
contains "hello" 'e'           # true
contains "hello" 'z'           # false
contains "" 'a'                # false
```

#### `take : Int -> List Char -> List Char`
Takes the first n characters.

```parlang
take 3 "hello"                 # "hel"
take 10 "hi"                   # "hi"
take 0 "hello"                 # ""
```

#### `drop : Int -> List Char -> List Char`
Drops the first n characters.

```parlang
drop 2 "hello"                 # "llo"
drop 10 "hi"                   # ""
drop 0 "hello"                 # "hello"
```

#### `strrev : List Char -> List Char -> List Char`
Reverses a string (requires accumulator).

```parlang
strrev Nil "hello"             # "olleh"
strrev Nil ""                  # ""
```

#### `char_at : Int -> List Char -> Option Char`
Gets character at index (0-based).

```parlang
char_at 0 "hello"              # Some('h')
char_at 4 "hello"              # Some('o')
char_at 10 "hello"             # None
```

#### `strcmp : List Char -> List Char -> Int`
Lexicographic comparison (-1, 0, 1).

```parlang
strcmp "abc" "abd"             # -1
strcmp "xyz" "xyz"             # 0
strcmp "zzz" "aaa"             # 1
```

#### `strmap : (Char -> Char) -> List Char -> List Char`
Maps a function over all characters.

```parlang
strmap toupper_char "hello"    # Transform each character
```

#### `strfilter : (Char -> Bool) -> List Char -> List Char`
Filters characters by predicate.

```parlang
strfilter isVowel "hello"      # "eo"
```

## Pattern Matching

Since strings are lists, pattern matching works naturally:

### Check Empty String

```parlang
match "" with
| Nil -> "empty"
| _ -> "not empty"
# Result: "empty"
```

### Check First Character

```parlang
match "hello" with
| Nil -> "empty"
| Cons 'h' _ -> "starts with h"
| Cons c _ -> "starts with other"
# Result: "starts with h"
```

### Decompose String

```parlang
match "hello" with
| Cons first (Cons second rest) -> (first, second, rest)
| _ -> (' ', ' ', Nil)
# Result: ('h', 'e', "llo")
```

## Advanced Usage

### Building Strings

You can construct strings character by character:

```parlang
type List a = Nil | Cons a (List a) in

Cons 'h' (Cons 'i' Nil)
# Equivalent to: "hi"
```

### String Interpolation (Manual)

```parlang
load "examples/string.par" in

let name = "Alice" in
let greeting = strcat "Hello, " (strcat name "!") in
greeting
# Result: "Hello, Alice!"
```

### Working with Lists

All list operations work:

```parlang
# Head of string
match "hello" with
| Cons c _ -> Some c
| Nil -> None
# Result: Some('h')

# Tail of string
match "hello" with
| Cons _ rest -> rest
| Nil -> Nil
# Result: "ello"
```

## Examples

### Word Counter

```parlang
type List a = Nil | Cons a (List a) in
type Option a = Some a | None in
load "examples/string.par" in

let is_space = fun c -> c == ' ' in

let count_words = rec count -> fun acc -> fun in_word -> fun s ->
  match s with
  | Nil -> if in_word then acc + 1 else acc
  | Cons c rest ->
      if is_space c
      then count acc false rest
      else count (if in_word then acc else acc + 1) true rest
in

count_words 0 false "hello world test"
# Result: 3
```

### String Trimming

```parlang
load "examples/string.par" in

let is_space = fun c -> c == ' ' in

let ltrim = rec trim -> fun s ->
  match s with
  | Nil -> Nil
  | Cons c rest ->
      if is_space c then trim rest else s
in

ltrim "  hello"
# Result: "hello"
```

## Performance Characteristics

Since strings are lists:
- **Access**: O(n) for character at index
- **Length**: O(n) - must traverse entire list
- **Concatenation**: O(n) where n is length of first string
- **Pattern matching on head**: O(1)

## Best Practices

1. **Use string library functions** instead of reinventing
2. **Pattern match for simple operations** (head, tail, empty check)
3. **Build strings left-to-right** for efficiency
4. **Consider using character arrays** for random access needs
5. **Load string.par once** at the start of your program

## Limitations

1. **No built-in string interpolation** - must use `strcat`
2. **No regular expressions** - would need separate library
3. **Case conversion limited** - requires char arithmetic
4. **Performance** - O(n) for most operations due to list structure

## Summary

ParLang's string support provides:
✅ Clean syntax with string literals
✅ Full Unicode support
✅ Type-safe through List Char
✅ Comprehensive standard library
✅ Pattern matching support
✅ Functional and immutable
✅ Easy to extend

For most text processing needs, ParLang's strings are both convenient and powerful!

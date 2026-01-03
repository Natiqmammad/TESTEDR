# 🚀 ApexForge NightScript (AFNS) - Complete Tutorial

**Author:** Natiq Mammadov — ApexForge  
**GitHub:** https://github.com/Natiqmammad  
**Version:** v1.0.0-alpha

![ApexForge Official Logo](assets/branding/apexforge_logo.png)

> **Welcome!** This is a comprehensive, progressive tutorial for learning ApexForge NightScript from the ground up. Whether you're a complete beginner or an experienced programmer, this guide will take you step-by-step through every feature of the language.

---

## 📖 Table of Contents

### Part 0: Introduction & Setup
- [0.1 ApexForge Tool (apexrc)](#01-apexforge-tool-apexrc)
- [0.2 Introduction to AFNS](#02-introduction-to-afns)
- [0.3 Getting Started](#03-getting-started)

### Part 1: Language Fundamentals
- [1. Syntax Basics](#1-syntax-basics)
- [2. Output Methods](#2-output-methods)
- [3. Variables](#3-variables)
- [4. Data Types](#4-data-types)
- [5. Type Casting](#5-type-casting)
- [6. Operators](#6-operators)

### Part 2: Working with Strings
- [7. Strings](#7-strings)

### Part 3: Math Operations
- [8. Math](#8-math)
- [9. Booleans](#9-booleans)

### Part 4: Control Flow
- [10. Conditions](#10-conditions)
- [11. Switch/Match](#11-switchmatch)

### Part 5: Loops
- [12. Loops](#12-loops)

### Part 6: Collections
- [13. Arrays](#13-arrays)
- [14. Vectors](#14-vectors)
- [15. Multi-dimensional Collections](#15-multi-dimensional-collections)
- [16. Maps/Dictionaries](#16-mapsdictionaries)
- [17. Sets](#17-sets)
- [18. Tuples](#18-tuples)

### Part 7: Functions
- [19. Functions](#19-functions)

### Part 8: Advanced Types
- [20. Structs](#20-structs)
- [21. Enums](#21-enums)
- [22. Traits](#22-traits)

### Part 9: Advanced Function Concepts
- [23. Method Parameters](#23-method-parameters)
- [24. Scopes](#24-scopes)
- [25. Arguments](#25-arguments)

### Part 10: Error Handling
- [26. Error Handling](#26-error-handling)
- [27. forge.error Library](#27-forgeerror-library)

### Part 11: File Operations
- [28. forge.fs (Filesystem)](#28-forgefs-filesystem)
- [29. forge.io (Input/Output)](#29-forgeio-inputoutput)

### Part 12: Networking
- [30. forge.net (Networking)](#30-forgenet-networking)

### Part 13: Asynchronous Programming
- [31. forge.async (Async Runtime)](#31-forgeasync-async-runtime)
- [32. forge.threads (Threading)](#32-forgethreads-threading)

### Part 14: Advanced Collections & Data Structures
- [33. HashMap](#33-hashmap)
- [34. Advanced Vector Operations](#34-advanced-vector-operations)

### Part 15: Platform-Specific Features
- [35. forge.gui.native (UI)](#35-forgeguinative-ui)
- [36. forge.android (Android Platform)](#36-forgeandroid-android-platform)

### Part 16: Advanced Features
- [37. forge.log (Logging)](#37-forgelog-logging)
- [38. Package System](#38-package-system)
- [39. Inline Assembly](#39-inline-assembly)
- [40. Memory Management](#40-memory-management)
- [41. Database Operations](#41-database-operations)
- [42. Cryptography](#42-cryptography)
- [43. Serialization](#43-serialization)

---

# Part 0: Introduction & Setup

## 0.1 ApexForge Tool (apexrc)

### What is apexrc?

`apexrc` is the official command-line tool for ApexForge NightScript. It provides everything you need to create, build, run, and manage AFNS projects.

### Installation

```bash
# Clone the repository
git clone https://github.com/Natiqmammad/TESTEDR.git
cd TESTEDR

# Build the compiler
cargo build --release

# The binary will be at target/release/apexrc
```

### Basic Commands

| Command | Description |
|---------|-------------|
| `apexrc new <name>` | Create a new project |
| `apexrc build` | Build the current project |
| `apexrc run` | Run the current project |
| `apexrc check` | Check code for errors |
| `apexrc registry` | Start local package registry |
| `apexrc publish` | Publish package to registry |
| `apexrc install` | Install dependencies |

### Project Structure

When you create a new project with `apexrc new hello`, you get:

```
hello/
├── Apex.toml          # Package configuration
├── src/
│   ├── main.afml      # Entry point
│   └── lib.afml       # Library code (optional)
└── target/            # Build artifacts (generated)
```

---

## 0.2 Introduction to AFNS

### What is ApexForge NightScript?

ApexForge NightScript (AFNS) is a **modern, high-performance programming language** designed for:

- **Systems Programming** - Low-level control with memory safety
- **Cross-Platform Development** - Write once, run everywhere
- **Async-First Architecture** - Built-in support for concurrent operations
- **High Performance** - Near-assembly speed with modern ergonomics

### Design Goals

✅ **Memory Safety** - Rust-level safety without garbage collection  
✅ **Performance** - 95% of Assembly performance  
✅ **Async-First** - Native async/await support  
✅ **Cross-Platform** - Linux, Android, embedded systems  
✅ **Developer Friendly** - Clean syntax, great tooling

### File Extension

All AFNS source files use the `.afml` extension:

```
main.afml
utils.afml
network.afml
```

### Entry Point

Every AFNS program starts with the `apex()` function:

```afml
fun apex() {
    // Your code here
}
```

For async programs:

```afml
async fun apex() {
    // Your async code here
}
```

---

## 0.3 Getting Started

### Your First Program

Create a file called `hello.afml`:

```afml
import forge.log as log;

fun apex() {
    log.info("Hello, ApexForge!");
}
```

### Running Your Program

Using apexrc:

```bash
apexrc new hello
cd hello
# Edit src/main.afml with the code above
apexrc run
```

Direct execution (during development):

```bash
cargo run -- hello.afml --run
```

### Understanding the Output

```
Hello, ApexForge!
```

Let's break down the program:

1. **`import forge.log as log;`** - Import the logging module
2. **`fun apex() { ... }`** - Define the entry point function
3. **`log.info("Hello, ApexForge!");`** - Print a message

---

# Part 1: Language Fundamentals

## 1. Syntax Basics

### Statement Structure

Every statement in AFNS ends with a semicolon (`;`):

```afml
let x = 10;
let name = "Alice";
log.info("Hello");
```

### Blocks

Blocks are enclosed in curly braces `{}`:

```afml
fun apex() {
    // This is a block
    let x = 5;
    
    if x > 0 {
        // This is another block
        log.info("Positive");
    }
}
```

### Comments

#### Single-Line Comments

```afml
// This is a single-line comment
let x = 10;  // Comment after code
```

#### Multi-Line Comments

```afml
/*
 * This is a multi-line comment
 * It can span multiple lines
 */
let y = 20;
```

### Identifiers

Identifiers (variable names, function names) must follow these rules:

✅ **Allowed:**
- Letters (A-Z, a-z)
- Digits (0-9) - but not as first character
- Underscore (_)

❌ **Not Allowed:**
- Unicode characters (∑, α, β, etc.)
- Special symbols (!, @, #, etc.)
- Keywords (fun, let, if, etc.)

**Examples:**

```afml
// ✅ Valid identifiers
let my_variable = 10;
let player1 = "Alice";
let _hidden = 42;
let userName = "Bob";

// ❌ Invalid identifiers
let 1player = "Error";     // Starts with digit
let user-name = "Error";   // Contains hyphen
let α = 10;                // Unicode not allowed
```

### Keywords

Reserved words that cannot be used as identifiers:

```
fun     async    await    return   if       else
while   for      in       switch   struct   enum
trait   impl     let      var      true     false
import  as       try      catch    match    break
continue pub     mod      use      type     const
```

---

## 2. Output Methods

### Using forge.log

The standard way to print output in AFNS is using the `forge.log` module.

### log.info()

The `log.info()` function prints messages to the console:

```afml
import forge.log as log;

fun apex() {
    log.info("This is a message");
}
```

**Output:**
```
This is a message
```

### Printing Text

```afml
import forge.log as log;

fun apex() {
    log.info("Hello, World!");
    log.info("Welcome to AFNS");
}
```

**Output:**
```
Hello, World!
Welcome to AFNS
```

### Printing Numbers

```afml
import forge.log as log;

fun apex() {
    log.info("Number:", 42);
    log.info("Pi:", 3.14159);
}
```

**Output:**
```
Number: 42
Pi: 3.14159
```

### Printing Multiple Values

Separate values with commas:

```afml
import forge.log as log;

fun apex() {
    let name = "Alice";
    let age = 25;
    log.info("Name:", name, "Age:", age);
}
```

**Output:**
```
Name: Alice Age: 25
```

---

## 3. Variables

### 3.1 General

Variables are containers for storing data. In AFNS, you declare variables using `let` or `var`.

### 3.2 Variable Declaration (let vs var)

#### Immutable Variables (let)

Use `let` to create an **immutable** variable (cannot be changed):

```afml
import forge.log as log;

fun apex() {
    let x = 10;
    log.info("x =", x);
    
    // x = 20;  // ❌ Error! Cannot reassign immutable variable
}
```

#### Mutable Variables (var)

Use `var` to create a **mutable** variable (can be changed):

```afml
import forge.log as log;

fun apex() {
    var x = 10;
    log.info("x =", x);
    
    x = 20;  // ✅ OK! Variable is mutable
    log.info("x =", x);
}
```

**Output:**
```
x = 10
x = 20
```

### 3.3 Printing Variables

```afml
import forge.log as log;

fun apex() {
    let name = "Bob";
    let score = 95;
    
    log.info("Player:", name);
    log.info("Score:", score);
}
```

### 3.4 Multiple Variables

Declare multiple variables one per line:

```afml
import forge.log as log;

fun apex() {
    let a = 10;
    let b = 20;
    let c = 30;
    
    log.info("Sum:", a + b + c);
}
```

### 3.5 Identifiers

Variable names should be descriptive and follow snake_case convention:

```afml
// ✅ Good names (snake_case)
let user_name = "Alice";
let total_score = 100;
let is_active = true;

// ⚠️ Also valid but not conventional
let userName = "Alice";    // camelCase
let TotalScore = 100;      // PascalCase

// ❌ Invalid names
let user-name = "Error";   // Contains hyphen
let 2fast = 10;            // Starts with digit
```

### 3.6 Constants

In AFNS, use `let` for constant values. The convention is to use UPPERCASE for true constants:

```afml
import forge.log as log;

fun apex() {
    let PI = 3.14159;
    let MAX_PLAYERS = 100;
    
    log.info("Pi:", PI);
    log.info("Max players:", MAX_PLAYERS);
}
```

### 3.7 Type Annotations

You can explicitly specify variable types using `::`:

```afml
import forge.log as log;

fun apex() {
    let x:: i32 = 10;           // 32-bit integer
    let y:: f64 = 3.14;         // 64-bit float
    let name:: str = "Alice";   // String
    let active:: bool = true;   // Boolean
    
    log.info("x:", x, "y:", y, "name:", name, "active:", active);
}
```

**Type Inference:**

AFNS can automatically infer types:

```afml
// These two are equivalent:
let x:: i32 = 10;
let x = 10;  // Type inferred as i32
```

---

## 4. Data Types

### 4.1 Overview

AFNS has two categories of data types:

1. **Primitive Types** - Basic built-in types
2. **Composite Types** - Complex types built from primitives

### 4.2 Numbers

#### 4.2.1 Integers

**Signed Integers** (can be negative):

| Type | Size | Range |
|------|------|-------|
| `i8` | 8 bits | -128 to 127 |
| `i16` | 16 bits | -32,768 to 32,767 |
| `i32` | 32 bits | -2,147,483,648 to 2,147,483,647 |
| `i64` | 64 bits | -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807 |
| `i128` | 128 bits | Very large range |

**Unsigned Integers** (only positive):

| Type | Size | Range |
|------|------|-------|
| `u8` | 8 bits | 0 to 255 |
| `u16` | 16 bits | 0 to 65,535 |
| `u32` | 32 bits | 0 to 4,294,967,295 |
| `u64` | 64 bits | 0 to 18,446,744,073,709,551,615 |
| `u128` | 128 bits | Very large range |

**Examples:**

```afml
import forge.log as log;

fun apex() {
    let age:: i32 = 25;
    let population:: u64 = 7800000000;
    let tiny:: i8 = -50;
    
    log.info("Age:", age);
    log.info("Population:", population);
    log.info("Tiny:", tiny);
}
```

#### 4.2.2 Floating Point

| Type | Size | Precision |
|------|------|-----------|
| `f32` | 32 bits | ~6-7 decimal digits |
| `f64` | 64 bits | ~15-16 decimal digits |

**Examples:**

```afml
import forge.log as log;

fun apex() {
    let pi:: f64 = 3.141592653589793;
    let temperature:: f32 = 98.6;
    
    log.info("Pi:", pi);
    log.info("Temperature:", temperature);
}
```

#### 4.2.3 Number Literals

You can use underscores for readability:

```afml
import forge.log as log;

fun apex() {
    let million = 1_000_000;
    let billion = 1_000_000_000;
    let hex = 0xFF;           // Hexadecimal
    let binary = 0b1010;      // Binary
    
    log.info("Million:", million);
    log.info("Billion:", billion);
}
```

### 4.3 Boolean (bool)

Boolean values represent true/false:

```afml
import forge.log as log;

fun apex() {
    let is_active:: bool = true;
    let is_done:: bool = false;
    
    log.info("Active:", is_active);
    log.info("Done:", is_done);
}
```

### 4.4 Characters (char)

A single character in single quotes:

```afml
import forge.log as log;

fun apex() {
    let letter:: char = 'A';
    let digit:: char = '5';
    let symbol:: char = '$';
    
    log.info("Letter:", letter);
}
```

**Note:** Only ASCII characters are allowed.

### 4.5 Strings (str)

Strings are sequences of characters in double quotes:

```afml
import forge.log as log;

fun apex() {
    let message:: str = "Hello, World!";
    let name:: str = "Alice";
    
    log.info(message);
    log.info("Name:", name);
}
```

#### Escape Sequences

| Sequence | Meaning |
|----------|---------|
| `\n` | Newline |
| `\t` | Tab |
| `\\` | Backslash |
| `\"` | Double quote |

```afml
import forge.log as log;

fun apex() {
    log.info("Line 1\nLine 2");
    log.info("Column 1\tColumn 2");
    log.info("She said, \"Hello!\"");
}
```

**Output:**
```
Line 1
Line 2
Column 1	Column 2
She said, "Hello!"
```

### 4.6 Non-Primitive Types

#### Fixed Arrays `[T;N]`

Arrays with a fixed size:

```afml
import forge.log as log;

fun apex() {
    let numbers:: [i32; 5] = [1, 2, 3, 4, 5];
    log.info("First:", numbers[0]);
}
```
Annotated arrays must match their declared length at runtime.

#### Vectors `vec<T>`

Dynamic arrays that can grow:

```afml
import forge.log as log;

fun apex() {
    let v:: vec<i32> = vec.new();
    vec.push(v, 10);
    vec.push(v, 20);
    
    log.info("Length:", vec.len(v));
}
```

#### Tuples `tuple(T1, T2, ...)`

Collections of different types:

```afml
import forge.log as log;

fun apex() {
    let person:: tuple(str, i32) = ("Alice", 25);
    log.info("Person:", person);
}
```
Tuple indexing (`person[0]`) is planned but not supported yet.

### 4.7 Special Types

#### option<T>

Represents an optional value (Some or None):

```afml
import forge.log as log;

fun apex() {
    let some_number = option.some(42);
    let no_number = option.none();
    
    log.info("Some:", some_number);
    log.info("None:", no_number);
}
```
`vec`, `option`, and `result` are available from the prelude; no extra imports are required.

#### result<T, E>

Represents success (Ok) or failure (Err):

```afml
import forge.log as log;

fun apex() {
    let success = result.ok(100);
    let failure = result.err("Something went wrong");
    
    log.info("Success:", success);
    log.info("Failure:", failure);
}
```

---

## 5. Type Casting

### Numeric casts (`as`)

Use `expr as Type` with checked conversions:

```afml
let x:: i32 = 10;
let y:: i64 = x as i64;      // widening is ok
let z:: f64 = y as f64;      // int to float is ok
```

If a cast would lose information, it errors:
- Narrowing out of range (e.g., `i64` large value to `i32`)
- Casting a float with fractional part to int
- Casting NaN/inf to int

### String to Number

String helpers return `result<T, str>`:

```afml
let ok = "123".to_i32();      // Ok(123)
let bad = "12x".to_i32();     // Err("invalid digit found in string")
let pi = "3.14".to_f64();     // Ok(3.14)
```

### Number to String

Numbers format automatically when printing (no extra call needed).

---

## 6. Operators

### 6.1 Arithmetic Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `+` | Addition | `5 + 3` = 8 |
| `-` | Subtraction | `5 - 3` = 2 |
| `*` | Multiplication | `5 * 3` = 15 |
| `/` | Division | `6 / 3` = 2 |
| `%` | Modulus (remainder) | `5 % 2` = 1 |

**Examples:**

```afml
import forge.log as log;

fun apex() {
    let a = 10;
    let b = 3;
    
    log.info("Addition:", a + b);      // 13
    log.info("Subtraction:", a - b);   // 7
    log.info("Multiplication:", a * b); // 30
    log.info("Division:", a / b);       // 3
    log.info("Modulus:", a % b);        // 1
}
```

### 6.2 Assignment Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `=` | Assign value | `x = 10` |

```afml
import forge.log as log;

fun apex() {
    var x = 10;
    log.info("x:", x);
    
    x = 20;
    log.info("x:", x);
}
```

### 6.3 Comparison Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `==` | Equal to | `5 == 5` → true |
| `!=` | Not equal to | `5 != 3` → true |
| `<` | Less than | `3 < 5` → true |
| `<=` | Less than or equal | `5 <= 5` → true |
| `>` | Greater than | `5 > 3` → true |
| `>=` | Greater than or equal | `5 >= 5` → true |

**Examples:**

```afml
import forge.log as log;

fun apex() {
    let x = 10;
    let y = 20;
    
    log.info("x == y:", x == y);  // false
    log.info("x != y:", x != y);  // true
    log.info("x < y:", x < y);    // true
    log.info("x > y:", x > y);    // false
}
```

### 6.4 Logical Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `&&` | Logical AND | `true && false` → false |
| `||` | Logical OR | `true || false` → true |
| `!` | Logical NOT | `!true` → false |

**Examples:**

```afml
import forge.log as log;

fun apex() {
    let a = true;
    let b = false;
    
    log.info("a && b:", a && b);  // false
    log.info("a || b:", a || b);  // true
    log.info("!a:", !a);          // false
}
```

`&&` and `||` are short-circuited:
- `false && expr` does not evaluate `expr`
- `true || expr` does not evaluate `expr`

### 6.5 Unary Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `-` | Negation | `-5` |
| `!` | Logical NOT | `!true` |

```afml
import forge.log as log;

fun apex() {
    let x = 10;
    let negative_x = -x;
    
    log.info("x:", x);               // 10
    log.info("negative_x:", negative_x);  // -10
    
    let is_true = true;
    log.info("!is_true:", !is_true);  // false
}
```

### 6.6 Operator Precedence

Operators are evaluated in this order (highest to lowest):

1. Calls, indexing, member access, casts (`f(x)`, `obj.field`, `arr[i]`, `expr as T`)
2. Unary (`-`, `!`)
3. Multiplication, Division, Modulus (`*`, `/`, `%`)
4. Addition, Subtraction (`+`, `-`)
5. Comparison (`<`, `<=`, `>`, `>=`)
6. Equality (`==`, `!=`)
7. Logical AND (`&&`)
8. Logical OR (`||`)
9. Range (`a..b` produces a half-open range for loops)
10. Assignment (`=`)

**Use parentheses to make precedence explicit:**

```afml
import forge.log as log;

fun apex() {
    let result1 = 5 + 3 * 2;      // 11 (multiplication first)
    let result2 = (5 + 3) * 2;    // 16 (addition first)
    
    log.info("Result 1:", result1);
    log.info("Result 2:", result2);
}
```

---

*Continue to [Part 2: Working with Strings →](#part-2-working-with-strings)*


# Part 2: Working with Strings

## 7. Strings

### 7.1 String Basics

Strings are sequences of characters enclosed in double quotes:

```afml
import forge.log as log;

fun apex() {
    let greeting = "Hello, World!";
    let name = "ApexForge";
    
    log.info(greeting);
    log.info("Language:", name);
}
```

### 7.2 String Concatenation

Currently, string concatenation is done by printing multiple values:

```afml
import forge.log as log;

fun apex() {
    let first_name = "John";
    let last_name = "Doe";
    
    log.info(first_name, last_name);  // John Doe
    log.info("Full name:", first_name, last_name);
}
```

### 7.3 Numbers and Strings

Numbers are automatically converted when printed with strings:

```afml
import forge.log as log;

fun apex() {
    let name = "Alice";
    let age = 25;
    let score = 95.5;
    
    log.info("Name:", name, "Age:", age, "Score:", score);
}
```

**Output:**
```
Name: Alice Age: 25 Score: 95.5
```

### 7.4 Special Characters

Use escape sequences for special characters:

| Sequence | Description | Example |
|----------|-------------|---------|
| `\n` | Newline | `"Line 1\nLine 2"` |
| `\t` | Tab | `"Name\tAge"` |
| `\\` | Backslash | `"Path: C:\\Users"` |
| `\"` | Double quote | `"She said \"Hi\""` |
| `\0` | Null byte | `"end\0"` |

**Examples:**

```afml
import forge.log as log;

fun apex() {
    // Newline
    log.info("First line\nSecond line");
    
    // Tab
    log.info("Name\tAge\tScore");
    log.info("Alice\t25\t95");
    
    // Quotes
    log.info("He said, \"Hello!\"");
    
    // Backslash
    log.info("File path: C:\\Users\\Documents");
}
```

**Output:**
```
First line
Second line
NameAgeScore
Alice2595
He said, "Hello!"
File path: C:\Users\Documents
```

### 7.5 String Methods

AFNS provides powerful string manipulation through the `forge.str` module.

#### len() - String Length

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let message = "Hello, World!";
    let length = str.len(message);
    
    log.info("Message:", message);
    log.info("Length:", length);  // 13
}
```

#### to_upper() - Convert to Uppercase

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let text = "hello world";
    let upper = str.to_upper(text);
    
    log.info("Original:", text);
    log.info("Uppercase:", upper);  // HELLO WORLD
}
```

#### to_lower() - Convert to Lowercase

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let text = "HELLO WORLD";
    let lower = str.to_lower(text);
    
    log.info("Original:", text);
    log.info("Lowercase:", lower);  // hello world
}
```

#### trim() - Remove Whitespace

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let text = "   Hello   ";
    let trimmed = str.trim(text);
    
    log.info("Original: [", text, "]");
    log.info("Trimmed: [", trimmed, "]");  // [Hello]
}
```

#### split() - Split String

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let text = "apple,banana,orange";
    let parts = str.split(text, ",");
    
    log.info("Parts:", parts);  // ["apple", "banana", "orange"]
}
```

#### replace() - Replace Substring

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let text = "Hello World";
    let replaced = str.replace(text, "World", "AFNS");
    
    log.info("Original:", text);
    log.info("Replaced:", replaced);  // Hello AFNS
}
```

#### find() - Find Substring Position

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let text = "Hello World";
    let pos = str.find(text, "World");
    
    log.info("Position:", pos);  // Some(6)
}
```

`str.find` returns `option<i64>`: `Some(index)` when found, `None` otherwise.

#### contains() - Check if Substring Exists

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let text = "Hello World";
    
    if str.contains(text, "World") {
        log.info("Found 'World'");
    }
    
    if !str.contains(text, "AFNS") {
        log.info("'AFNS' not found");
    }
}
```

#### starts_with() - Check Prefix

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let filename = "document.pdf";
    
    if str.starts_with(filename, "doc") {
        log.info("Filename starts with 'doc'");
    }
}
```

#### ends_with() - Check Suffix

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let filename = "document.pdf";
    
    if str.ends_with(filename, ".pdf") {
        log.info("This is a PDF file");
    }
}
```

#### Complete String Example

```afml
import forge.log as log;
import forge.str as str;

fun apex() {
    let text = "  Hello, ApexForge NightScript!  ";
    
    // Get length
    log.info("Length:", str.len(text));
    
    // Trim whitespace
    let trimmed = str.trim(text);
    log.info("Trimmed:", trimmed);
    
    // Convert case
    log.info("Upper:", str.to_upper(trimmed));
    log.info("Lower:", str.to_lower(trimmed));
    
    // Search
    if str.contains(trimmed, "ApexForge") {
        log.info("Contains 'ApexForge'");
    }
    
    // Replace
    let replaced = str.replace(trimmed, "NightScript", "AFNS");
    log.info("Replaced:", replaced);
}
```

---

# Part 3: Math Operations

## 8. Math

### 8.1 Basic Math

AFNS provides comprehensive math operations through the `forge.math` module.

#### Mathematical Constants

```afml
import forge.log as log;
import forge.math as math;

fun apex() {
    let pi = math.pi();
    log.info("Pi:", pi);  // 3.14159...
}
```

#### sqrt() - Square Root

```afml
import forge.log as log;
import forge.math as math;

fun apex() {
    let x = 16.0;
    let result = math.sqrt(x)?;  // result<f64, str> on domain errors
    
    log.info("Square root of", x, "is", result);  // 4.0
}
```

#### Basic Arithmetic Example

```afml
import forge.log as log;
import forge.math as math;

fun apex() {
    let radius = 5.0;
    let pi = math.pi();
    let area = pi * radius * radius;
    let hyp = math.sqrt(25)?; // Ok(5.0)
    
    log.info("Radius:", radius);
    log.info("Area:", area);  // 78.539...
    log.info("Hyp:", hyp);
}
```

### 8.2 Trigonometry

Common trigonometric functions:

```afml
import forge.log as log;
import forge.math as math;

fun apex() {
    let angle = 0.0;  // radians
    
    let sine = math.sin(angle);
    let cosine = math.cos(angle);
    let tangent = math.tan(angle);
    
    let arc_sine = math.asin(0.5)?;   // domain-checked: returns result
    let arc_cos = math.acos(0.5)?;    // domain-checked: returns result
    let atan = math.atan2(1.0, 1.0);  // plain float
    
    log.info("sin:", sine, "cos:", cosine, "tan:", tangent);
    log.info("asin:", arc_sine, "acos:", arc_cos, "atan2:", atan);
}
```

### 8.3 Advanced Functions

Available helpers (all in `forge.math`):

- `pow(base, exp)` → float
- `abs(x)` → same type (int/float)
- `floor(x)`, `ceil(x)`, `round(x)` → int stays int; float returns float
- `exp(x)`, `ln(x)`, `log10(x)`, `log2(x)` (logs return `result<f64, str>` on invalid input)
- `min(a, b)`, `max(a, b)`, `clamp(x, min, max)` (matching numeric types only)
- Trig: `sin`, `cos`, `tan`, `asin` (result), `acos` (result), `atan`, `atan2`
- `sqrt(x)` → `result<f64, str>` (negative input errors)

**Example:**

```afml
import forge.log as log;
import forge.math as math;

fun apex() {
    let x = 2.7;
    let absolute = math.abs(-5.0);
    let power = math.pow(2.0, 3.0);       // 8.0
    let ceiling = math.ceil(3.2);         // 4.0
    let floor = math.floor(3.8);          // 3.0
    let safe_ln = math.ln(0.0);           // Err("ln domain error...")
    
    log.info("abs:", absolute);
    log.info("pow:", power);
    log.info("ceil/floor:", ceiling, floor);
    log.info("ln(0):", safe_ln);
}
```

---

## 9. Booleans

### Boolean Values

Booleans represent truth values:

```afml
import forge.log as log;

fun apex() {
    let is_active = true;
    let is_closed = false;
    
    log.info("Active:", is_active);
    log.info("Closed:", is_closed);
}
```

### Boolean Expressions

Comparisons produce boolean values:

```afml
import forge.log as log;

fun apex() {
    let age = 18;
    let is_adult = age >= 18;
    
    log.info("Age:", age);
    log.info("Is adult:", is_adult);  // true
}
```

### Using Booleans in Conditions

```afml
import forge.log as log;

fun apex() {
    let has_permission = true;
    let is_authenticated = true;
    
    if has_permission && is_authenticated {
        log.info("Access granted");
    } else {
        log.info("Access denied");
    }
}
```

Conditions must be `bool`. Numbers/strings are not truthy:

```afml
fun apex() {
    if 1 { }          // runtime error: expected bool
}
```

### Boolean Operators

```afml
import forge.log as log;

fun apex() {
    let a = true;
    let b = false;
    
    log.info("a AND b:", a && b);   // false
    log.info("a OR b:", a || b);    // true
    log.info("NOT a:", !a);         // false
    log.info("NOT b:", !b);         // true
}
```

---

# Part 4: Control Flow

## 10. Conditions

### 10.1 If Statements

The `if` statement executes code based on a condition:

```afml
import forge.log as log;

fun apex() {
    let age = 18;
    
    if age >= 18 {
        log.info("You are an adult");
    }
}
```

`if` conditions must be `bool`; non-boolean conditions raise a runtime error.

`else if` chains associate with the nearest `if`:

```afml
fun apex() {
    let score = 72;
    if score >= 90 {
        log.info("A");
    } else if score >= 80 {
        log.info("B");
    } else if score >= 70 {
        log.info("C");
    } else {
        log.info("D or below");
    }
}
```

### 10.2 Else

The `else` clause runs when the condition is false:

```afml
import forge.log as log;

fun apex() {
    let age = 15;
    
    if age >= 18 {
        log.info("You are an adult");
    } else {
        log.info("You are a minor");
    }
}
```

**Output:**
```
You are a minor
```

### 10.3 Else If

Chain multiple conditions:

```afml
import forge.log as log;

fun apex() {
    let score = 85;
    
    if score >= 90 {
        log.info("Grade: A");
    } else if score >= 80 {
        log.info("Grade: B");
    } else if score >= 70 {
        log.info("Grade: C");
    } else if score >= 60 {
        log.info("Grade: D");
    } else {
        log.info("Grade: F");
    }
}
```

**Output:**
```
Grade: B
```

### 10.4 Nested If

You can nest if statements:

```afml
import forge.log as log;

fun apex() {
    let age = 20;
    let has_license = true;
    
    if age >= 18 {
        if has_license {
            log.info("You can drive");
        } else {
            log.info("You need a license");
        }
    } else {
        log.info("You are too young");
    }
}
```

### 10.5 Block Expressions

Blocks can return values:

```afml
import forge.log as log;

fun apex() {
    let x = 10;
    
    let result = if x > 5 {
        "Greater than 5"
    } else {
        "Less than or equal to 5"
    };
    
    log.info("Result:", result);
}
```

**Complete Conditional Example:**

```afml
import forge.log as log;

fun apex() {
    let temperature = 25;
    let is_raining = false;
    
    if temperature > 30 {
        log.info("It's hot outside");
    } else if temperature > 20 {
        if is_raining {
            log.info("It's warm but rainy");
        } else {
            log.info("Perfect weather!");
        }
    } else if temperature > 10 {
        log.info("It's cool outside");
    } else {
        log.info("It's cold outside");
    }
}
```

---

## 11. Switch/Match

### 11.1 Switch Syntax

The `switch` statement allows pattern matching:

```afml
import forge.log as log;

fun apex() {
    let day = 3;
    
    switch day {
        1 -> log.info("Monday"),
        2 -> log.info("Tuesday"),
        3 -> log.info("Wednesday"),
        4 -> log.info("Thursday"),
        5 -> log.info("Friday"),
        6 -> log.info("Saturday"),
        7 -> log.info("Sunday"),
        _ -> log.info("Invalid day"),
    }
}
```

The scrutinee is evaluated once; the first matching arm runs. If no arm matches and there is no `_`, the switch does nothing.

**Output:**
```
Wednesday
```

### 11.2 Patterns

Switch can match various patterns:

```afml
import forge.log as log;

fun apex() {
    let number = 42;
    
    switch number {
        0 -> log.info("Zero"),
        1 -> log.info("One"),
        42 -> log.info("The answer"),
        _ -> log.info("Some other number"),
    }
}
```

Supported today: literal patterns (numbers, bools, chars, strings) and the wildcard `_`. Enum/path patterns are planned but not enabled yet.

### 11.3 Wildcard (_)

The underscore `_` matches anything:

```afml
import forge.log as log;

fun apex() {
    let status_code = 404;
    
    switch status_code {
        200 -> log.info("OK"),
        404 -> log.info("Not Found"),
        500 -> log.info("Server Error"),
        _ -> log.info("Unknown status"),
    }
}
```

**Output:**
```
Not Found
```

### 11.4 Pattern Matching with Enums (planned)

Enum pattern matching will be added later; for now use literal cases and `_`:

```afml
import forge.log as log;

enum Status {
    Ok,
    Error(msg:: str),
    Warning(code:: i32),
}

fun print_status(s:: Status) {
    // Planned syntax (not yet enabled in runtime):
    // switch s {
    //     Ok -> log.info("Everything is fine"),
    //     Error(msg) -> log.info("Error:", msg),
    //     Warning(code) -> log.info("Warning code:", code),
    //     _ -> log.info("Unknown status"),
    // }
}

fun apex() {
    print_status(Status::Ok);
    print_status(Status::Error("File not found"));
    print_status(Status::Warning(101));
}
```

---

# check: Guard-Based Branching

`check` is an expression or statement for readable guard chains.

With a target value:

```afml
import forge.log as log;

fun apex() {
    let v = 15;
    let label = check v {
        1 -> "one",
        it > 10 -> "big",
        _ -> "other",
    };
    log.info("label:", label);
}
```

Guard-only form:

```afml
fun apex() {
    let status = check {
        2 + 2 == 5 -> "impossible",
        2 + 2 == 4 -> "ok",
        _ -> "default",
    };
}
```

Rules:
- Conditions/guards must evaluate to `bool`.
- If a target is provided, it is available as `it` inside guards/arms.
- `_` is the wildcard/default. Missing `_` causes a runtime “non-exhaustive” error.

---

# Part 5: Loops

## 12. Loops

### 12.1 While Loop

The `while` loop repeats code while a condition is true:

```afml
import forge.log as log;

fun apex() {
    var i = 0;
    
    while i < 5 {
        log.info("Count:", i);
        i = i + 1;
    }
    
    log.info("Done!");
}
```

**Output:**
```
Count: 0
Count: 1
Count: 2
Count: 3
Count: 4
Done!
```

Loop conditions must be `bool`; using numbers or strings will raise a runtime error.

### 12.2 Loop Control

Use while loops for flexible iteration:

```afml
import forge.log as log;

fun apex() {
    var sum = 0;
    var i = 1;
    
    while i <= 10 {
        sum = sum + i;
        i = i + 1;
    }
    
    log.info("Sum of 1 to 10:", sum);  // 55
}
```

### 12.3 Break Statement

The `break` statement exits a loop:

```afml
import forge.log as log;

fun apex() {
    var i = 0;
    
    while true {
        if i >= 5 {
            break;
        }
        log.info("i:", i);
        i = i + 1;
    }
    
    log.info("Loop ended");
}
```

**Output:**
```
i: 0
i: 1
i: 2
i: 3
i: 4
Loop ended
```

### 12.4 Continue Statement

The `continue` statement skips to the next iteration:

```afml
import forge.log as log;

fun apex() {
    var i = 0;
    
    while i < 10 {
        i = i + 1;
        
        if i % 2 == 0 {
            continue;  // Skip even numbers
        }
        
        log.info("Odd number:", i);
    }
}
```

**Output:**
```
Odd number: 1
Odd number: 3
Odd number: 5
Odd number: 7
Odd number: 9
```

### 12.5 For Loop

The `for` loop iterates over collections:

```afml
import forge.log as log;

fun apex() {
    let numbers = vec.new();
    vec.push(numbers, 10);
    vec.push(numbers, 20);
    vec.push(numbers, 30);
    
    for num in numbers {
        log.info("Number:", num);
    }
}
```

**Output:**
```
Number: 10
Number: 20
Number: 30
```

You can also iterate ranges directly with the `..` operator (start inclusive, end exclusive):

```afml
import forge.log as log;

fun apex() {
    for i in 0..3 {
        log.info("i:", i);  // 0, then 1, then 2
    }
}
```

`for` works with `vec`, arrays, and ranges (`start` inclusive, `end` exclusive). `break` exits the nearest loop; `continue` skips to the next iteration. A `do-while` loop is planned but not implemented yet.

### 12.6 Nested Loops

Loops can be nested:

```afml
import forge.log as log;

fun apex() {
    var i = 1;
    
    while i <= 3 {
        var j = 1;
        while j <= 3 {
            log.info("i:", i, "j:", j);
            j = j + 1;
        }
        i = i + 1;
    }
}
```

**Output:**
```
i: 1 j: 1
i: 1 j: 2
i: 1 j: 3
i: 2 j: 1
i: 2 j: 2
i: 2 j: 3
i: 3 j: 1
i: 3 j: 2
i: 3 j: 3
```

**Complete Loop Example:**

```afml
import forge.log as log;

fun apex() {
    // Countdown
    var count = 10;
    while count > 0 {
        log.info("Countdown:", count);
        count = count - 1;
    }
    log.info("Blast off!");
    
    // Sum even numbers
    var sum = 0;
    var i = 0;
    while i <= 20 {
        if i % 2 == 0 {
            sum = sum + i;
        }
        i = i + 1;
    }
    log.info("Sum of even numbers 0-20:", sum);
}
```

---

*Continue to [Part 6: Collections →](#part-6-collections)*


# Part 6: Collections

## 13. Arrays

### 13.1 Fixed Arrays

Arrays store multiple values of the same type with a fixed size:

```afml
import forge.log as log;

fun apex() {
    let numbers:: [i32; 5] = [1, 2, 3, 4, 5];
    log.info("Array:", numbers);
}
```

### 13.2 Array Declaration

Declare arrays with type and size:

```afml
import forge.log as log;

fun apex() {
    let scores:: [i32; 3] = [95, 87, 92];
    let names:: [str; 2] = ["Alice", "Bob"];
    
    log.info("Scores:", scores);
    log.info("Names:", names);
}
```

### 13.3 Array Access (Indexing)

Access elements using square brackets (0-indexed):

```afml
import forge.log as log;

fun apex() {
    let fruits:: [str; 3] = ["apple", "banana", "orange"];
    
    log.info("First fruit:", fruits[0]);   // apple
    log.info("Second fruit:", fruits[1]);  // banana
    log.info("Third fruit:", fruits[2]);   // orange
}
```

### 13.4 Array Length

Arrays have a fixed length:

```afml
import forge.log as log;

fun apex() {
    let numbers:: [i32; 5] = [10, 20, 30, 40, 50];
    log.info("Array length:", numbers.len()); // 5
}
```

**Bounds:** `numbers[i]` is checked at runtime. Accessing an index `< 0` or `>= len` raises an error:  
`array index out of bounds: idx=10 len=5`

### 13.5 Nested Arrays

Arrays nest naturally: `[[i32; 2]; 2]` and literal `[[1, 2], [3, 4]]`. Index step-by-step: `grid[1][0]` -> `3`. Each dimension is bounds-checked.

---

## 14. Vectors

### 14.1 Vector Basics

Vectors are dynamic arrays that can grow:

```afml
import forge.log as log;

fun apex() {
    let v:: vec<i32> = vec.new();
    log.info("Created empty vector");
}
```

### 14.2 Adding Elements (push)

Use `vec.push()` to add elements:

```afml
import forge.log as log;

fun apex() {
    let numbers = vec.new();
    
    vec.push(numbers, 10);
    vec.push(numbers, 20);
    vec.push(numbers, 30);
    
    log.info("Vector:", numbers);
}
```

### 14.3 Reading/Writing Elements

- `vec.get(v, idx) -> option<T>` (returns `option.none()` when out of bounds)
- `vec.set(v, idx, value) -> result<(), str>` (error if out of bounds or type mismatch)
- `vec.pop(v) -> option<T>`
- `vec.len(v) -> i64`
- Nesting works: `vec<vec<i32>>` lets you push inner vectors and access them with `vec.get`.
- Tuples: `(a, b, c)` with type `tuple(T1, T2, ...)`; index with `t[0]`, `t[1]`. Out-of-bounds raises `tuple index out of bounds`.

All bounds checks are enforced at runtime; errors are descriptive results, never panics.

---

## 15. Maps & Sets

- `map.new()`, `map.put/get/remove/contains_key/keys/values/items`, length via `map.len`.
- Keys supported: `str`, integers, `bool` (others error with a clear message).
- Values and keys keep type tags; wrong types return `result.err`.
- `set.new()`, `set.insert` (returns `result<bool, str>`), `set.remove/contains/len`, `set.to_vec`, `set.union/intersection/difference` (results).
- Set elements support `str`, integers, and `bool`; other element types error.

### 14.3 Removing Elements (pop)

Use `vec.pop()` to remove the last element:

```afml
import forge.log as log;

fun apex() {
    let stack = vec.new();
    
    vec.push(stack, 1);
    vec.push(stack, 2);
    vec.push(stack, 3);
    
    let last = vec.pop(stack);
    log.info("Popped:", last);      // 3
    log.info("Remaining:", stack);  // [1, 2]
}
```

### 14.4 Vector Methods

#### len() - Get Length

```afml
import forge.log as log;

fun apex() {
    let items = vec.new();
    vec.push(items, "apple");
    vec.push(items, "banana");
    vec.push(items, "orange");
    
    let count = vec.len(items);
    log.info("Count:", count);  // 3
}
```

#### sort() - Sort Elements

```afml
import forge.log as log;

fun apex() {
    let numbers = vec.new();
    vec.push(numbers, 30);
    vec.push(numbers, 10);
    vec.push(numbers, 20);
    
    log.info("Before sort:", numbers);
    vec.sort(numbers);
    log.info("After sort:", numbers);  // [10, 20, 30]
}
```

#### reverse() - Reverse Order

```afml
import forge.log as log;

fun apex() {
    let items = vec.new();
    vec.push(items, 1);
    vec.push(items, 2);
    vec.push(items, 3);
    
    log.info("Before:", items);
    vec.reverse(items);
    log.info("After:", items);  // [3, 2, 1]
}
```

#### insert() - Insert at Position

```afml
import forge.log as log;

fun apex() {
    let items = vec.new();
    vec.push(items, "first");
    vec.push(items, "third");
    
    vec.insert(items, 1, "second");
    log.info("Items:", items);  // ["first", "second", "third"]
}
```

#### remove() - Remove at Position

```afml
import forge.log as log;

fun apex() {
    let items = vec.new();
    vec.push(items, "a");
    vec.push(items, "b");
    vec.push(items, "c");
    
    let removed = vec.remove(items, 1);
    log.info("Removed:", removed);  // "b"
    log.info("Remaining:", items);  // ["a", "c"]
}
```

#### extend() - Append Another Vector

```afml
import forge.log as log;

fun apex() {
    let v1 = vec.new();
    vec.push(v1, 1);
    vec.push(v1, 2);
    
    let v2 = vec.new();
    vec.push(v2, 3);
    vec.push(v2, 4);
    
    vec.extend(v1, v2);
    log.info("Extended:", v1);  // [1, 2, 3, 4]
}
```

### 14.5 Vectors in Loops

Iterate over vectors with for loops:

```afml
import forge.log as log;

fun apex() {
    let fruits = vec.new();
    vec.push(fruits, "apple");
    vec.push(fruits, "banana");
    vec.push(fruits, "orange");
    
    for fruit in fruits {
        log.info("Fruit:", fruit);
    }
}
```

**Complete Vector Example:**

```afml
import forge.log as log;

fun apex() {
    // Create a shopping list
    let shopping_list = vec.new();
    
    // Add items
    vec.push(shopping_list, "milk");
    vec.push(shopping_list, "eggs");
    vec.push(shopping_list, "bread");
    
    log.info("Shopping list:", shopping_list);
    log.info("Items:", vec.len(shopping_list));
    
    // Remove last item
    let last_item = vec.pop(shopping_list);
    log.info("Removed:", last_item);
    
    // Add more items
    vec.push(shopping_list, "cheese");
    vec.push(shopping_list, "butter");
    
    // Sort alphabetically
    vec.sort(shopping_list);
    log.info("Sorted list:", shopping_list);
}
```

---

## 15. Multi-dimensional Collections

### Nested Vectors

Create vectors of vectors for 2D data:

```afml
import forge.log as log;

fun apex() {
    // Create a 2x3 matrix
    let matrix = vec.new();
    
    let row1 = vec.new();
    vec.push(row1, 1);
    vec.push(row1, 2);
    vec.push(row1, 3);
    
    let row2 = vec.new();
    vec.push(row2, 4);
    vec.push(row2, 5);
    vec.push(row2, 6);
    
    vec.push(matrix, row1);
    vec.push(matrix, row2);
    
    log.info("Matrix:", matrix);
    // Access element: matrix[0][0] = 1
}
```

### Matrix Operations

```afml
import forge.log as log;

fun apex() {
    // Create a 3x3 identity matrix
    let identity = vec.new();
    
    var i = 0;
    while i < 3 {
        let row = vec.new();
        var j = 0;
        while j < 3 {
            if i == j {
                vec.push(row, 1);
            } else {
                vec.push(row, 0);
            }
            j = j + 1;
        }
        vec.push(identity, row);
        i = i + 1;
    }
    
    log.info("Identity matrix:", identity);
}
```

---

## 16. Maps/Dictionaries

### 16.1 Map Creation

Maps store key-value pairs:

```afml
import forge.log as log;

fun apex() {
    let dict:: map<str, i32> = map.new();
    log.info("Created empty map");
}
```

### 16.2 Put/Get Operations

#### put() - Add or Update Entry

```afml
import forge.log as log;

fun apex() {
    let ages = map.new();
    
    map.put(ages, "Alice", 25);
    map.put(ages, "Bob", 30);
    map.put(ages, "Carol", 28);
    
    log.info("Ages:", ages);
}
```

#### get() - Retrieve Value

```afml
import forge.log as log;

fun apex() {
    let scores = map.new();
    
    map.put(scores, "Alice", 95);
    map.put(scores, "Bob", 87);
    
    let alice_score = map.get(scores, "Alice");
    log.info("Alice's score:", alice_score);  // 95
}
```

### 16.3 Keys and Values

#### keys() - Get All Keys

```afml
import forge.log as log;

fun apex() {
    let capitals = map.new();
    map.put(capitals, "France", "Paris");
    map.put(capitals, "Japan", "Tokyo");
    map.put(capitals, "USA", "Washington");
    
    let country_list = map.keys(capitals);
    log.info("Countries:", country_list);
}
```

#### values() - Get All Values

```afml
import forge.log as log;

fun apex() {
    let capitals = map.new();
    map.put(capitals, "France", "Paris");
    map.put(capitals, "Japan", "Tokyo");
    
    let city_list = map.values(capitals);
    log.info("Cities:", city_list);
}
```

### 16.4 Map Methods

#### len() - Number of Entries

```afml
import forge.log as log;

fun apex() {
    let phonebook = map.new();
    map.put(phonebook, "Alice", "555-1234");
    map.put(phonebook, "Bob", "555-5678");
    
    let count = map.len(phonebook);
    log.info("Entries:", count);  // 2
}
```

#### remove() - Delete Entry

```afml
import forge.log as log;

fun apex() {
    let config = map.new();
    map.put(config, "debug", "true");
    map.put(config, "verbose", "false");
    
    map.remove(config, "verbose");
    log.info("Config:", config);
}
```

**Complete Map Example:**

```afml
import forge.log as log;

fun apex() {
    // Create student grades database
    let grades = map.new();
    
    // Add students
    map.put(grades, "Alice", 95);
    map.put(grades, "Bob", 87);
    map.put(grades, "Carol", 92);
    
    // Get a grade
    let alice_grade = map.get(grades, "Alice");
    log.info("Alice's grade:", alice_grade);
    
    // Update a grade
    map.put(grades, "Bob", 90);
    
    // Show all students
    let students = map.keys(grades);
    log.info("Students:", students);
    
    // Count entries
    log.info("Total students:", map.len(grades));
}
```

---

## 17. Sets

### 17.1 Set Creation

Sets store unique values:

```afml
import forge.log as log;

fun apex() {
    let unique_numbers:: set<i32> = set.new();
    log.info("Created empty set");
}
```

### 17.2 Insert/Remove

#### insert() - Add Element

```afml
import forge.log as log;

fun apex() {
    let tags = set.new();
    
    set.insert(tags, "rust");
    set.insert(tags, "afns");
    set.insert(tags, "programming");
    
    // Duplicate insertions are ignored
    set.insert(tags, "rust");
    
    log.info("Tags:", tags);
}
```

#### remove() - Delete Element

```afml
import forge.log as log;

fun apex() {
    let items = set.new();
    set.insert(items, "a");
    set.insert(items, "b");
    set.insert(items, "c");
    
    set.remove(items, "b");
    log.info("Items:", items);
}
```

### 17.3 Contains

Check if element exists:

```afml
import forge.log as log;

fun apex() {
    let allowed = set.new();
    set.insert(allowed, "admin");
    set.insert(allowed, "user");
    
    if set.contains(allowed, "admin") {
        log.info("Admin access allowed");
    }
    
    if !set.contains(allowed, "guest") {
        log.info("Guest access not in set");
    }
}
```

### 17.4 Set Operations

#### len() - Number of Elements

```afml
import forge.log as log;

fun apex() {
    let colors = set.new();
    set.insert(colors, "red");
    set.insert(colors, "green");
    set.insert(colors, "blue");
    
    log.info("Number of colors:", set.len(colors));  // 3
}
```

**Complete Set Example:**

```afml
import forge.log as log;

fun apex() {
    // Track unique visitors
    let visitors = set.new();
    
    // Add visitors
    set.insert(visitors, "alice@example.com");
    set.insert(visitors, "bob@example.com");
    set.insert(visitors, "carol@example.com");
    
    // Try to add duplicate
    set.insert(visitors, "alice@example.com");
    
    log.info("Unique visitors:", set.len(visitors));  // 3
    
    // Check if visited
    if set.contains(visitors, "bob@example.com") {
        log.info("Bob has visited");
    }
}
```

---

## 18. Tuples

### Heterogeneous Collections

Tuples hold different types together:

```afml
import forge.log as log;

fun apex() {
    let person:: tuple(str, i32, bool) = ("Alice", 25, true);
    log.info("Person:", person);
}
```

### Tuple Access

Access elements by index:

```afml
import forge.log as log;

fun apex() {
    let point:: tuple(f64, f64) = (3.5, 7.2);
    
    // Access will be available in future releases
    // let x = point[0];
    // let y = point[1];
    
    log.info("Point:", point);
}
```

### Tuple Examples

```afml
import forge.log as log;

fun apex() {
    // RGB color
    let color:: tuple(i32, i32, i32) = (255, 128, 0);
    
    // Person data
    let employee:: tuple(str, i32, str) = ("John Doe", 35, "Engineer");
    
    // Coordinates with metadata
    let location:: tuple(f64, f64, str) = (40.7128, -74.0060, "New York");
    
    log.info("Color:", color);
    log.info("Employee:", employee);
    log.info("Location:", location);
}
```

---

*Continue to [Part 7: Functions →](#part-7-functions)*


# Part 7: Functions

## 19. Functions

### 19.1 Function Declaration

Functions are declared with the `fun` keyword:

```afml
import forge.log as log;

fun greet() {
    log.info("Hello from a function!");
}

fun apex() {
    greet();
}
```

### 19.2 Parameters

Functions can accept parameters:

```afml
import forge.log as log;

fun greet(name:: str) {
    log.info("Hello,", name);
}

fun apex() {
    greet("Alice");
    greet("Bob");
}
```

**Output:**
```
Hello, Alice
Hello, Bob
```

### 19.3 Return Values

Functions can return values using `->` and `return`:

```afml
import forge.log as log;

fun add(a:: i32, b:: i32) -> i32 {
    return a + b;
}

fun apex() {
    let result = add(5, 3);
    log.info("5 + 3 =", result);  // 8
}
```

### 19.4 Function Calls

Call functions by name with parentheses:

```afml
import forge.log as log;

fun square(x:: i32) -> i32 {
    return x * x;
}

fun apex() {
    let num = 7;
    let result = square(num);
    log.info(num, "squared is", result);  // 49
}
```

### 19.5 Multiple Parameters

Functions can have multiple parameters:

```afml
import forge.log as log;

fun calculate_area(width:: f64, height:: f64) -> f64 {
    return width * height;
}

fun apex() {
    let area = calculate_area(5.0, 3.0);
    log.info("Area:", area);  // 15.0
}
```

**Complete Function Example:**

```afml
import forge.log as log;

fun is_even(n:: i32) -> bool {
    return n % 2 == 0;
}

fun factorial(n:: i32) -> i32 {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}

fun max(a:: i32, b:: i32) -> i32 {
    if a > b {
        return a;
    } else {
        return b;
    }
}

fun apex() {
    log.info("Is 4 even?", is_even(4));      // true
    log.info("Is 7 even?", is_even(7));      // false
    
    log.info("5! =", factorial(5));          // 120
    
    log.info("Max of 10 and 20:", max(10, 20));  // 20
}
```

---

# Part 8: Advanced Types

## 20. Structs

### 20.1 Struct Definition

Structs group related data together:

```afml
import forge.log as log;

struct Point {
    x:: f64,
    y:: f64,
}

fun apex() {
    log.info("Point struct defined");
}
```

### 20.2 Struct Instantiation

Create struct instances:

```afml
import forge.log as log;

struct Person {
    name:: str,
    age:: i32,
}

fun apex() {
    let alice = Person {
        name: "Alice",
        age: 25,
    };
    
    log.info("Person created:", alice);
}
```

### 20.3 Field Access

Access struct fields with dot notation:

```afml
import forge.log as log;

struct Rectangle {
    width:: f64,
    height:: f64,
}

fun apex() {
    let rect = Rectangle {
        width: 10.0,
        height: 5.0,
    };
    
    log.info("Width:", rect.width);
    log.info("Height:", rect.height);
}
```

### 20.4 Methods for Structs (impl blocks)

Define methods using `impl`:

```afml
import forge.log as log;

struct Circle {
    radius:: f64,
}

impl Circle {
    fun area(self) -> f64 {
        let pi = 3.14159;
        return pi * self.radius * self.radius;
    }
}

fun apex() {
    let c = Circle { radius: 5.0 };
    let area = c.area();
    log.info("Circle area:", area);
}
```

**Complete Struct Example:**

```afml
import forge.log as log;

struct Student {
    name:: str,
    id:: i32,
    gpa:: f64,
}

impl Student {
    fun is_honor_roll(self) -> bool {
        return self.gpa >= 3.5;
    }
    
    fun display(self) {
        log.info("Student:", self.name);
        log.info("ID:", self.id);
        log.info("GPA:", self.gpa);
    }
}

fun apex() {
    let student = Student {
        name: "Alice",
        id: 12345,
        gpa: 3.8,
    };
    
    student.display();
    
    if student.is_honor_roll() {
        log.info("Honor roll student!");
    }
}
```

---

## 21. Enums

### 21.1 Enum Definition

Enums define a type with multiple variants:

```afml
import forge.log as log;

enum Color {
    Red,
    Green,
    Blue,
}

fun apex() {
    log.info("Color enum defined");
}
```

### 21.2 Enum Variants

Use enum variants:

```afml
import forge.log as log;

enum Direction {
    North,
    South,
    East,
    West,
}

fun apex() {
    let dir = Direction::North;
    log.info("Direction:", dir);
}
```

### 21.3 Variants with Data

Enums can hold data:

```afml
import forge.log as log;

enum Message {
    Quit,
    Text(content:: str),
    Move(x:: i32, y:: i32),
}

fun apex() {
    let msg1 = Message::Quit;
    let msg2 = Message::Text("Hello");
    let msg3 = Message::Move(10, 20);
    
    log.info("Messages created");
}
```

### 21.4 Methods for Enums

Define methods on enums:

```afml
import forge.log as log;

enum Status {
    Ok,
    Error(msg:: str),
}

impl Status {
    fun is_ok(self) -> bool {
        switch self {
            Ok -> return true,
            _ -> return false,
        }
    }
}

fun apex() {
    let s = Status::Ok;
    if s.is_ok() {
        log.info("Status is OK");
    }
}
```

**Complete Enum Example:**

```afml
import forge.log as log;

enum Result {
    Success(value:: i32),
    Failure(error:: str),
}

fun divide(a:: i32, b:: i32) -> Result {
    if b == 0 {
        return Result::Failure("Division by zero");
    }
    return Result::Success(a / b);
}

fun apex() {
    let r1 = divide(10, 2);
    let r2 = divide(10, 0);
    
    switch r1 {
        Success(val) -> log.info("Result:", val),
        Failure(err) -> log.info("Error:", err),
        _ -> log.info("Unknown"),
    }
    
    switch r2 {
        Success(val) -> log.info("Result:", val),
        Failure(err) -> log.info("Error:", err),
        _ -> log.info("Unknown"),
    }
}
```

---

## 22. Traits

### 22.1 Trait Definition

Traits define shared behavior:

```afml
trait Printable {
    fun print(self);
}
```

### 22.2 Implementing Traits

Implement traits for types:

```afml
import forge.log as log;

trait Drawable {
    fun draw(self);
}

struct Circle {
    radius:: f64,
}

impl Drawable for Circle {
    fun draw(self) {
        log.info("Drawing circle with radius", self.radius);
    }
}

fun apex() {
    let c = Circle { radius: 5.0 };
    c.draw();
}
```

### 22.3 Trait Bounds

Constrain generic types with traits:

```afml
// Future feature
trait Comparable {
    fun compare(self, other:: Self) -> i32;
}
```

---

# Part 9: Advanced Function Concepts

## 23. Method Parameters

### Passing by Value

By default, parameters are passed by value (copied):

```afml
import forge.log as log;

fun modify(x:: i32) {
    var y = x;
    y = y + 10;
    log.info("Inside function:", y);
}

fun apex() {
    let num = 5;
    modify(num);
    log.info("Outside function:", num);  // Still 5
}
```

### References

Use references to avoid copying (future feature):

```afml
// Future: &T for immutable reference, &mut T for mutable
```

---

## 24. Scopes

### Lexical Scoping

Variables are scoped to their block:

```afml
import forge.log as log;

fun apex() {
    let x = 10;
    
    {
        let y = 20;
        log.info("Inner scope - x:", x, "y:", y);
    }
    
    log.info("Outer scope - x:", x);
    // y is not accessible here
}
```

### Variable Shadowing

Inner scopes can shadow outer variables:

```afml
import forge.log as log;

fun apex() {
    let x = 10;
    log.info("Outer x:", x);
    
    {
        let x = 20;
        log.info("Inner x:", x);  // Shadows outer x
    }
    
    log.info("Outer x again:", x);
}
```

**Output:**
```
Outer x: 10
Inner x: 20
Outer x again: 10
```

### Lifetime of Variables

Variables live until their scope ends:

```afml
import forge.log as log;

fun apex() {
    {
        let temp = 100;
        log.info("temp:", temp);
    }  // temp is destroyed here
    
    // temp is not accessible here
    log.info("temp is gone");
}
```

---

## 25. Arguments

### Function Arguments

Pass values to functions:

```afml
import forge.log as log;

fun process(data:: str, count:: i32, verbose:: bool) {
    if verbose {
        log.info("Processing", data, count, "times");
    }
    
    var i = 0;
    while i < count {
        log.info(data);
        i = i + 1;
    }
}

fun apex() {
    process("Hello", 3, true);
}
```

### Command-Line Arguments

Access program arguments (future feature):

```afml
// Future: forge.os.args()
```

---

# Part 10: Error Handling

## 26. Error Handling

### 26.1 Result Type

The `result<T, E>` type represents success or failure:

```afml
import forge.log as log;

fun divide(a:: i32, b:: i32) -> result<i32, str> {
    if b == 0 {
        return result.err("Division by zero");
    }
    return result.ok(a / b);
}

fun apex() {
    let r1 = divide(10, 2);
    let r2 = divide(10, 0);
    
    log.info("Result 1:", r1);  // Ok(5)
    log.info("Result 2:", r2);  // Err("Division by zero")
}
```

### 26.2 Option Type

The `option<T>` type represents an optional value:

```afml
import forge.log as log;

fun find_index(items:: vec<str>, target:: str) -> option<i32> {
    var i = 0;
    let len = vec.len(items);
    
    while i < len {
        if items[i] == target {
            return option.some(i);
        }
        i = i + 1;
    }
    
    return option.none();
}

fun apex() {
    let fruits = vec.new();
    vec.push(fruits, "apple");
    vec.push(fruits, "banana");
    vec.push(fruits, "orange");
    
    let idx = find_index(fruits, "banana");
    log.info("Index:", idx);  // Some(1)
    
    let not_found = find_index(fruits, "grape");
    log.info("Not found:", not_found);  // None
}
```

### 26.3 Error Propagation (? operator)

Use `?` to propagate errors:

```afml
import forge.log as log;

fun safe_divide(a:: i32, b:: i32) -> result<i32, str> {
    if b == 0 {
        return result.err("Division by zero");
    }
    return result.ok(a / b);
}

fun calculate(x:: i32, y:: i32, z:: i32) -> result<i32, str> {
    let step1 = safe_divide(x, y)?;  // Propagate errors
    let step2 = safe_divide(step1, z)?;
    return result.ok(step2);
}

fun apex() {
    let r1 = calculate(20, 2, 5);  // Ok(2)
    let r2 = calculate(20, 0, 5);  // Err("Division by zero")
    
    log.info("Result 1:", r1);
    log.info("Result 2:", r2);
}
```

### 26.4 Try/Catch Blocks

Handle errors with try/catch:

```afml
import forge.log as log;

fun risky_operation() -> result<i32, str> {
    return result.err("Something went wrong!");
}

fun apex() {
    try {
        let value = risky_operation()?;
        log.info("Success:", value);
    } catch(e) {
        log.info("Caught error:", e);
    }
}
```

### 26.5 Panic

Use `panic` for unrecoverable errors:

```afml
import forge.log as log;

fun apex() {
    let critical_value = 0;
    
    if critical_value == 0 {
        panic("Critical error: value cannot be zero!");
    }
    
    log.info("This won't execute");
}
```

**Complete Error Handling Example:**

```afml
import forge.log as log;

fun read_config(key:: str) -> result<str, str> {
    if key == "port" {
        return result.ok("8080");
    } else if key == "host" {
        return result.ok("localhost");
    }
    return result.err("Key not found");
}

fun get_config_value(key:: str) -> option<str> {
    let config_result = read_config(key);
    
    // Convert result to option (future helper)
    switch config_result {
        Ok(val) -> return option.some(val),
        _ -> return option.none(),
    }
}

fun apex() {
    // Using result
    let port_result = read_config("port");
    log.info("Port:", port_result);
    
    let invalid_result = read_config("invalid");
    log.info("Invalid:", invalid_result);
    
    // Error propagation
    let port = read_config("port")?;
    log.info("Port value:", port);
}
```

---

## 27. forge.error Library

### Error Types

AFNS provides error utilities:

```afml
import forge.error as error;

// Future features:
// error.new("custom message")
// error.throw("error message")
```

### Custom Errors

Define custom error types:

```afml
// Future: Custom error enums
enum FileError {
    NotFound(path:: str),
    PermissionDenied,
    InvalidFormat,
}
```

---

*Continue to [Part 11: File Operations →](#part-11-file-operations)*


# Part 11: File Operations

## 28. forge.fs (Filesystem)

The `forge.fs` module provides comprehensive file and directory operations.

### 28.1 Reading Files

#### read_to_string() - Read File as String

```afml
import forge.log as log;
import forge.fs as fs;

fun apex() {
    let content = fs.read_to_string("config.txt")?;
    log.info("File content:", content);
}
```

#### read_bytes() - Read File as Bytes

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    let data = fs.read_bytes("image.png")?;
    log.info("File size:", vec.len(data), "bytes");
}
```

### 28.2 Writing Files

#### write_string() - Write String to File

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    fs.write_string("output.txt", "Hello, AFNS!")?;
    log.info("File written successfully");
}
```

#### append_string() - Append to File

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    fs.write_string("log.txt", "Line 1\n")?;
    fs.append_string("log.txt", "Line 2\n")?;
    fs.append_string("log.txt", "Line 3\n")?;
    
    log.info("Log file updated");
}
```

### 28.3 File Operations

#### exists() - Check if File Exists

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    if fs.exists("data.txt") {
        log.info("File exists");
    } else {
        log.info("File not found");
    }
}
```

#### copy_file() - Copy File

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    fs.copy_file("source.txt", "backup.txt")?;
    log.info("File copied");
}
```

#### move() / rename() - Move or Rename File

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    fs.move("old_name.txt", "new_name.txt")?;
    log.info("File renamed");
}
```

#### remove_file() - Delete File

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    fs.remove_file("temp.txt")?;
    log.info("File deleted");
}
```

### 28.4 Directory Operations

#### create_dir() - Create Directory

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    fs.create_dir("new_folder")?;
    log.info("Directory created");
}
```

#### create_dir_all() - Create Nested Directories

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    fs.create_dir_all("parent/child/grandchild")?;
    log.info("Nested directories created");
}
```

#### remove_dir() - Remove Empty Directory

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    fs.remove_dir("empty_folder")?;
    log.info("Directory removed");
}
```

#### remove_dir_all() - Remove Directory and Contents

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    fs.remove_dir_all("folder_with_files")?;
    log.info("Directory and contents removed");
}
```

#### read_dir() - List Directory Contents

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    let entries = fs.read_dir(".")?;
    
    for entry in entries {
        log.info("Entry:", entry);
    }
}
```

### 28.5 Path Manipulation

#### join() - Join Path Components

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    let path = fs.join("users", "documents");
    log.info("Path:", path);  // users/documents
}
```

#### dirname() - Get Directory Name

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    let dir = fs.dirname("/home/user/file.txt");
    log.info("Directory:", dir);  // /home/user
}
```

#### basename() - Get File Name

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    let name = fs.basename("/home/user/file.txt");
    log.info("Basename:", name);  // file.txt
}
```

#### extension() - Get File Extension

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    let ext = fs.extension("document.pdf");
    log.info("Extension:", ext);  // Some("pdf")
}
```

### 28.6 Metadata

#### metadata() - Get File Info

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    let meta = fs.metadata("file.txt")?;
    
    log.info("Size:", meta.size);
    log.info("Is file:", meta.is_file);
    log.info("Is directory:", meta.is_dir);
    log.info("Read-only:", meta.readonly);
}
```

**Complete File Operations Example:**

```afml
import forge.fs as fs;
import forge.log as log;

fun apex() {
    // Create working directory
    let work_dir = "workspace";
    fs.ensure_dir(work_dir)?;
    
    // Write a file
    let file_path = fs.join(work_dir, "data.txt");
    fs.write_string(file_path, "Initial content\n")?;
    
    // Append more data
    fs.append_string(file_path, "Additional line\n")?;
    
    // Read and display
    let content = fs.read_to_string(file_path)?;
    log.info("Content:", content);
    
    // Get metadata
    let meta = fs.metadata(file_path)?;
    log.info("File size:", meta.size, "bytes");
    
    // Copy file
    let backup_path = fs.join(work_dir, "data_backup.txt");
    fs.copy_file(file_path, backup_path)?;
    
    // List directory
    let entries = fs.read_dir(work_dir)?;
    log.info("Files created:", vec.len(entries));
    
    // Cleanup
    fs.remove_dir_all(work_dir)?;
    log.info("Cleanup complete");
}
```

---

## 29. forge.io (Input/Output)

### 29.1 File I/O

Basic file operations are in `forge.fs` (see above).

### 29.2 Network I/O

Network I/O is available in `forge.net` (see below).

### 29.3 Memory I/O

Read and write memory buffers:

```afml
// Future features for memory-mapped I/O
```

### 29.4 Streams

Stream-based I/O (future feature):

```afml
// Future: io.stream.read(), io.stream.write()
```

---

# Part 12: Networking

## 30. forge.net (Networking)

### 30.1 HTTP Client

#### GET Request

```afml
// Future HTTP support
import forge.net.http as http;
import forge.log as log;

async fun apex() {
    let response = await http.get("https://api.example.com/data")?;
    let body = await response.text();
    log.info("Response:", body);
}
```

#### POST Request

```afml
// Future
async fun apex() {
    let data = map.new();
    map.put(data, "username", "alice");
    
    let response = await http.post("https://api.example.com/login", json=data)?;
    log.info("Status:", response.status());
}
```

### 30.2 TCP

#### TCP Connect

```afml
import forge.net as net;
import forge.log as log;

fun apex() {
    let socket = net.tcp_connect("127.0.0.1:8080")?;
    
    net.tcp_send(socket, "Hello, server!")?;
    let response = net.tcp_recv(socket, 1024)?;
    
    log.info("Response:", response);
    net.close_socket(socket)?;
}
```

#### TCP Listen

```afml
import forge.net as net;
import forge.log as log;

fun apex() {
    let listener = net.tcp_listen("127.0.0.1:8080")?;
    log.info("Server listening on port 8080");
    
    let client = net.tcp_accept(listener)?;
    let data = net.tcp_recv(client, 1024)?;
    
    log.info("Received:", data);
    net.close_socket(client)?;
    net.close_listener(listener)?;
}
```

### 30.3 UDP

#### UDP Send/Receive

```afml
import forge.net as net;
import forge.log as log;

fun apex() {
    let socket = net.udp_bind("127.0.0.1:9000")?;
    
    net.udp_send_to(socket, "Hello!", "127.0.0.1:9001")?;
    
    let packet = net.udp_recv_from(socket, 1024)?;
    log.info("Received from:", packet.from);
    log.info("Data:", packet.data);
    
    net.close_socket(socket)?;
}
```

### 30.4 WebSockets

```afml
// Future WebSocket support
async fun apex() {
    let ws = await ws.connect("wss://example.com/socket")?;
    await ws.send("Hello");
    let msg = await ws.recv();
    log.info("Message:", msg);
}
```

### 30.5 DNS

```afml
// Future DNS support
fun apex() {
    let ips = dns.lookup("example.com")?;
    log.info("IP addresses:", ips);
}
```

---

# Part 13: Asynchronous Programming

## 31. forge.async (Async Runtime)

### 31.1 Async Functions

Define async functions with `async fun`:

```afml
import forge.log as log;
import forge.async as async;

async fun fetch_data() -> async str {
    await async.sleep(100);
    return "data loaded";
}

async fun apex() {
    log.info("Starting...");
    let data = await fetch_data();
    log.info("Data:", data);
}
```

### 31.2 Await Expression

Use `await` to wait for async operations:

```afml
import forge.log as log;
import forge.async as async;

async fun download(url:: str) -> async str {
    log.info("Downloading:", url);
    await async.sleep(200);
    return "Downloaded content";
}

async fun apex() {
    let content = await download("https://example.com");
    log.info(content);
}
```

### 31.3 Futures

Async functions return futures:

```afml
import forge.log as log;
import forge.async as async;

async fun long_task() -> async i32 {
    await async.sleep(500);
    return 42;
}

async fun apex() {
    let future = long_task();
    log.info("Task started");
    
    let result = await future;
    log.info("Result:", result);
}
```

### 31.4 async.sleep()

Pause execution for a duration:

```afml
import forge.log as log;
import forge.async as async;

async fun apex() {
    log.info("Start");
    await async.sleep(1000);  // Sleep for 1 second
    log.info("After 1 second");
}
```

### 31.5 async.timeout()

Execute with timeout:

```afml
import forge.log as log;
import forge.async as async;

async fun apex() {
    let callback = fun() {
        log.info("Timeout reached!");
    };
    
    await async.timeout(500, callback);
}
```

### 31.6 async.parallel()

Run tasks in parallel:

```afml
import forge.log as log;
import forge.async as async;

async fun task1() -> async i32 {
    await async.sleep(100);
    return 1;
}

async fun task2() -> async i32 {
    await async.sleep(100);
    return 2;
}

async fun apex() {
    let tasks = vec.new();
    vec.push(tasks, task1());
    vec.push(tasks, task2());
    
    let results = await async.parallel(tasks);
    log.info("Results:", results);
}
```

### 31.7 async.race()

Wait for first to complete:

```afml
import forge.log as log;
import forge.async as async;

async fun slow_task() -> async str {
    await async.sleep(1000);
    return "slow";
}

async fun fast_task() -> async str {
    await async.sleep(100);
    return "fast";
}

async fun apex() {
    let tasks = vec.new();
    vec.push(tasks, slow_task());
    vec.push (tasks, fast_task());
    
    let winner = await async.race(tasks);
    log.info("Winner:", winner);  // "fast"
}
```

### 31.8 async.all()

Wait for all tasks to complete:

```afml
import forge.log as log;
import forge.async as async;

async fun apex() {
    let tasks = vec.new();
    
    // Add multiple async tasks
    var i = 0;
    while i < 5 {
        vec.push(tasks, async.sleep(100 * i));
        i = i + 1;
    }
    
    await async.all(tasks);
    log.info("All tasks complete");
}
```

---

## 32. forge.threads (Threading)

### Thread Creation

```afml
// Future threading support
import forge.threads as threads;

fun apex() {
    let handle = threads.spawn(fun() {
        log.info("Running in thread");
    });
    
    threads.join(handle);
}
```

### Thread Synchronization

```afml
// Future: Mutex, RwLock, Channels
```

---

# Part 14: Advanced Collections & Data Structures

## 33. HashMap

HashMap operations (covered in Part 6 - Maps)

```afml
import forge.log as log;

fun apex() {
    let phonebook = map.new();
    
    map.put(phonebook, "Alice", "555-1234");
    map.put(phonebook, "Bob", "555-5678");
    
    let alice_number = map.get(phonebook, "Alice");
    log.info("Alice's number:", alice_number);
}
```

---

## 34. Advanced Vector Operations

Advanced vector methods (covered in Part 6):

- `vec.sort()` - Sort elements
- `vec.reverse()` - Reverse order
- `vec.insert()` - Insert at position
- `vec.remove()` - Remove at position
- `vec.extend()` - Append another vector

---

# Part 15: Platform-Specific Features

## 35. forge.gui.native (UI)

### 35.1 Widget System

Create native UI widgets:

```afml
// Future UI support
import forge.gui.native as ui;

fun apex() {
    ui.window("My App", fun(ctx) {
        ctx.text("Hello, World!");
        ctx.button("Click Me", fun() {
            log.info("Button clicked!");
        });
    });
}
```

### 35.2 Layout (Column, Row)

```afml
fun apex() {
    ui.column([
        ui.text("Header"),
        ui.row([
            ui.button("Left"),
            ui.button("Right"),
        ]),
    ]);
}
```

### 35.3 Common Widgets

- `Text` - Display text
- `Button` - Clickable button
- `TextField` - Text input
- `Image` - Display images
- `Column` / `Row` - Layout
- `Scaffold` - App structure
- `AppBar` - Top bar
- `ListView` - Scrollable list

### 35.4 Event Handling

```afml
fun apex() {
    ui.button("Submit", fun() {
        log.info("Form submitted");
    });
}
```

---

## 36. forge.android (Android Platform)

### 36.1 Activity Lifecycle

```afml
import forge.android.app as app;

struct MyActivity {}

impl app::Activity for MyActivity {
    fun on_create(ctx:: app::Context) {
        ctx.show_toast("App started!");
    }
    
    fun on_resume(ctx:: app::Context) {
        log.info("App resumed");
    }
}

fun apex() {
    app.run(MyActivity {});
}
```

### 36.2 Permissions

```afml
import forge.android.app as app;

fun apex() {
    if app.permissions.is_granted("CAMERA") {
        log.info("Camera permission granted");
    } else {
        app.permissions.request("CAMERA");
    }
}
```

### 36.3 Intents

```afml
import forge.android.app as app;

fun apex() {
    app.intent.send("android.intent.action.VIEW", url="https://google.com");
}
```

### 36.4 Services

```afml
import forge.android.app as app;

struct MyService {}

fun apex() {
    app.service.start(MyService {});
}
```

### 36.5 Storage

```afml
import forge.android.app as app;

fun apex() {
    let internal = app.storage.get_internal_path();
    let external = app.storage.get_external_path();
    
    log.info("Internal:", internal);
    log.info("External:", external);
}
```

### 36.6 Java FFI

Call Java code from AFNS:

```afml
@ffi("java:android.os.Build")
extern "Java" fun MODEL() -> str;

fun apex() {
    let device_model = MODEL();
    log.info("Device:", device_model);
}
```

---

# Part 16: Advanced Features

## 37. forge.log (Logging)

### Log Levels

```afml
import forge.log as log;

fun apex() {
    log.info("Information message");
    log.warn("Warning message");      // Future
    log.error("Error message");       // Future
    log.debug("Debug message");       // Future
}
```

### Formatting

```afml
import forge.log as log;

fun apex() {
    let user = "Alice";
    let score = 95;
    
    log.info("User:", user, "scored", score, "points");
}
```

---

## 38. Package System

### 38.1 Imports

Import modules:

```afml
import forge;              // Import forge module
import forge.log as log;   // Import with alias
import forge.fs::read_to_string;  // Import specific item
import math.utils as utils;       // Dotted path, Python style
```

### 38.2 Module Structure

Organize code in modules:

```
myproject/
  Apex.toml
  src/
    main.afml
    utils.afml
    math/
      mod.afml
      calculus.afml
```

Resolution order: stdlib (`src/forge`) → vendored packages (`target/vendor/afml/...`) → global packages (`~/.apex/packages/...`) → local project `src/`. For `import a.b.c`, files tried in order: `a/b/c.afml`, then `a/b/c/mod.afml`, then `a/b/c/lib.afml`. `import path::member as alias` loads the module then binds a single exported item.

### 38.3 Registry (apexrc)

Use the package registry:

```bash
# Start registry
apexrc registry

# Publish package
apexrc publish

# Install package
apexrc install some-package@1.0.0

# Add dependency
apexrc add forge.fs@^1.0
```

---

## 39. Inline Assembly

### Assembly Blocks

Write inline assembly:

```afml
import forge.mem as mem;
import forge.log as log;

fun apex() {
    var result = 0;
    
    assembly {
        mov rax, 42
        mov [result], rax
    }
    
    log.info("Result from assembly:", result);
}
```

---

## 40. Memory Management

### 40.1 Ownership

AFNS uses Rust-like ownership:

```afml
// One owner, moves on assignment
let v1 = vec.new();
let v2 = v1;  // v1 is now invalid, v2 owns the vector
```

### 40.2 Borrowing

#### Immutable References (&T)

```afml
// Future: &T for immutable borrow
```

#### Mutable References (&mut T)

```afml
// Future: &mut T for mutable borrow
```

### 40.3 Lifetimes

Lifetimes ensure references are valid:

```afml
// Future lifetime annotations
```

### 40.4 Smart Pointers

#### box<T>

Heap allocation:

```afml
// Future: box<T>
```

#### rc<T>

Reference counting:

```afml
// Future: rc<T>
```

#### arc<T>

Atomic reference counting (thread-safe):

```afml
// Future: arc<T>
```

### 40.5 Raw Pointers

Unsafe pointer operations:

```afml
// Future: ptr<T>, ptr_mut<T>
```

### 40.6 forge.mem Library

Manual memory operations:

```afml
import forge.mem as mem;

fun apex() {
    let ptr = mem.alloc(64);      // Allocate 64 bytes
    mem.set(ptr, 0, 64);          // Zero the memory
    mem.free(ptr);                 // Free memory
}
```

---

## 41. Database Operations

### 41.1 forge.db

Database module:

```afml
import forge.db as db;
```

### 41.2 SQL (SQLite, PostgreSQL)

```afml
import forge.db as db;
import forge.log as log;

fun apex() {
    let conn = db.open("sqlite", "database.db")?;
    
    db.exec(conn, "CREATE TABLE users (id INTEGER, name TEXT)")?;
    db.exec(conn, "INSERT INTO users VALUES (1, 'Alice')")?;
    
    let rows = db.query(conn, "SELECT * FROM users")?;
    log.info("Users:", rows);
    
    db.close(conn)?;
}
```

### 41.3 NoSQL (Redis, MongoDB)

```afml
import forge.db as db;

fun apex() {
    let conn = db.open("redis", "redis://localhost")?;
    
    db.set(conn, "key1", "value1")?;
    let value = db.get(conn, "key1")?;
    
    log.info("Value:", value);
    db.close(conn)?;
}
```

---

## 42. Cryptography

### 42.1 forge.crypto

Cryptographic operations (future):

```afml
import forge.crypto as crypto;
```

### 42.2 Hashing

```afml
// Future
fun apex() {
    let hash = crypto.sha256("Hello, World!");
    log.info("SHA-256:", hash);
}
```

### 42.3 Encryption

```afml
// Future
fun apex() {
    let key = crypto.generate_key();
    let encrypted = crypto.aes.encrypt("secret", key);
    let decrypted = crypto.aes.decrypt(encrypted, key);
}
```

### 42.4 Signing

```afml
// Future
fun apex() {
    let keypair = crypto.ed25519.generate();
    let signature = crypto.ed25519.sign("message", keypair.private);
    let valid = crypto.ed25519.verify("message", signature, keypair.public);
}
```

---

## 43. Serialization

### 43.1 forge.serde

Serialization module (future):

```afml
import forge.serde as serde;
```

### 43.2 JSON

```afml
// Future
fun apex() {
    let obj = map.new();
    map.put(obj, "name", "Alice");
    map.put(obj, "age", 25);
    
    let json = serde.json.encode(obj);
    log.info("JSON:", json);
    
    let decoded = serde.json.decode(json);
}
```

### 43.3 YAML

```afml
// Future
fun apex() {
    let yaml = serde.yaml.encode(obj);
    let decoded = serde.yaml.decode(yaml);
}
```

### 43.4 Binary Formats

```afml
// Future: MessagePack-like binary serialization
fun apex() {
    let binary = serde.bin.encode(obj);
    let decoded = serde.bin.decode(binary);
}
```

---

# 🎓 Conclusion

Congratulations! You've completed the comprehensive ApexForge NightScript tutorial. You now know:

✅ **Fundamentals** - Variables, types, operators, control flow  
✅ **Collections** - Arrays, vectors, maps, sets, tuples  
✅ **Functions** - Declaration, parameters, returns, async  
✅ **Advanced Types** - Structs, enums, traits  
✅ **Error Handling** - Result, option, try/catch, panic  
✅ **File Operations** - Reading, writing, directories, paths  
✅ **Networking** - TCP, UDP, HTTP (future)  
✅ **Async Programming** - async/await, futures, parallel execution  
✅ **Platform Features** - Android, UI (future)  
✅ **Advanced Topics** - Memory management, databases, crypto (future)

## Phase 2 Quick Reference (Sets, Tuples, Structs, Enums, Traits)

```afml
// Sets (str/int/bool keys)
let s = set.new();
set.insert(s, "a");
let has_a = set.contains(s, "a");    // true
let merged = set.union(s, s);        // result<set<str>, str>

// Tuples
let t = ("hi", 42);
let first = t[0];                    // "hi"

// Structs + methods
struct User { name:: str, age:: i32 }
impl User { fun greet(self) -> str { return self.name; } }
let u = User { name: "Ada", age: 30 };
let msg = u.greet();                 // "Ada"

// Enums + switch binding
enum Status { Ok, Error(str) }
let s = Status::Error("fail");
switch s {
    Ok -> print("ok");
    Error(msg) -> print(msg);        // "fail"
    _ -> print("other");
}

// Traits (static dispatch)
trait Display { fun to_string(self) -> str; }
impl Display for User { fun to_string(self) -> str { return self.name; } }
print(Display::to_string(u));        // "Ada"
```

## Scopes & Shadowing
- Scope chain: global/module → function → block → switch arm → try/catch.
- The same name cannot be declared twice in a single scope (`let x = 1; let x = 2;` is an error).
- Inner scopes may shadow outer names: `{ let x = 2; }` while outer `x` remains unchanged.
- Pattern binders in `switch` and catch variables only live inside their arm/block.

## Method Receiver Rules
- Methods inside `impl Type { ... }` must start with `self` or `self_mut` as the first parameter.
- `self_mut` requires the receiver binding to be mutable; otherwise runtime error: `cannot borrow immutable value as mutable (method requires self_mut)`.
- Method calls `obj.method(a, b)` implicitly pass the receiver as the first argument.

## Argument Evaluation Order
- Receiver is evaluated first, then arguments left-to-right.
- Builtins like `print/log` accept varargs; other calls must match arity.
- Current runtime uses copy semantics for values; no ownership/borrowing model yet.

## Error Handling Basics
- `result<T, str>` and `option<T>` are built-in; constructors via `result.ok/err`, `option.some/none`.
- `?` on `result`: `Ok(v)` unwraps, `Err(e)` returns the error from the current function.
- `?` on `option`: `Some(v)` unwraps; `None` returns `option.none()` only if the function returns `option<T>`, otherwise runtime error.
- `try { ... } catch(e) { ... }` catches `forge.error.throw` and runtime errors as strings.
- `panic(msg)` aborts with `panic: msg` (caught by `try` as a string in this phase).

## forge.error
- `forge.error.new(code, msg) -> "[code] msg"`
- `forge.error.wrap(err, ctx) -> "ctx: err"`
- `forge.error.throw(msg)` raises and is caught by `try/catch`; uncaught throw aborts execution.

## Next Steps

1. **Build a Project** - Create your own AFNS application
2. **Read Examples** - Explore `examples/` directory
3. **Join Community** - Contribute to ApexForge
4. **Check Documentation** - See README.md for full spec

## Useful Resources

- **GitHub Repository:** https://github.com/Natiqmammad/TESTEDR
- **apexrc Tool:** Built-in compiler and package manager
- **Examples:** `examples/` directory for sample code

## Package Commands

```bash
apexrc new myproject      # Create project
apexrc build              # Build project
apexrc run                # Run project
apexrc check              # Check for errors
apexrc add <package>      # Add dependency
apexrc publish            # Publish to registry
```

---

**Happy Coding with ApexForge NightScript! 🚀**

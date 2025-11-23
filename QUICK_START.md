# 🚀 NightScript Quick Start Guide

**ApexForge NightScript (AFNS) v1.0.0-alpha**

---

## Installation & Setup

### Prerequisites
- Rust 1.70+ (install from https://rustup.rs/)
- Git

### Clone & Build
```bash
git clone https://github.com/your-repo/NightScript.git
cd NightScript
cargo build --release
```

### Run Examples
```bash
# Execute a program
cargo run -- examples/basic.afml --run

# Show parsed AST
cargo run -- examples/basic.afml --ast

# Show tokens
cargo run -- examples/basic.afml --tokens
```

---

## Language Basics

### Hello World
```afml
import forge.log as log;

fun apex() {
    log.info("Hello, NightScript!");
}
```

### Variables & Types
```afml
fun apex() {
    let x = 42;           // integer
    let y = 3.14;         // float
    let s = "hello";      // string
    let b = true;         // boolean
    var z = 10;           // mutable variable
    z = 20;               // reassignment
}
```

### Functions
```afml
fun add(a:: i32, b:: i32) -> i32 {
    return a + b;
}

async fun fetch_data() -> async str {
    await async.sleep(100);
    return "data";
}

fun apex() {
    let result = add(5, 3);
    log.info("Result: ", result);
}
```

### Control Flow
```afml
fun apex() {
    // If/else
    if x > 0 {
        log.info("positive");
    } else {
        log.info("non-positive");
    }
    
    // While loop
    var i = 0;
    while i < 10 {
        log.info(i);
        i = i + 1;
    }
}
```

### Collections
```afml
fun apex() {
    // Vector
    let v = vec.new();
    vec.push(v, 1);
    vec.push(v, 2);
    let len = vec.len(v);
    
    // Result
    let ok = result.ok(42);
    let err = result.err("error");
    
    // Option
    let some = option.some(10);
    let none = option.none();
}
```

### Error Handling
```afml
fun safe_divide(a:: i32, b:: i32) -> result<i32, str> {
    if b == 0 {
        return result.err("Division by zero");
    }
    return result.ok(a / b);
}

fun apex() {
    let res = safe_divide(10, 2)?;  // Propagate error with ?
    log.info("Result: ", res);
}
```

### Async/Await
```afml
async fun load_data() {
    log.info("Loading...");
    await async.sleep(100);
    log.info("Done!");
}

async fun apex() {
    await load_data();
}
```

### Strings
```afml
fun apex() {
    let s = "hello";
    let len = str.len(s);
    let upper = str.to_upper(s);
    let lower = str.to_lower(s);
    let trimmed = str.trim(s);
}
```

### Math
```afml
fun apex() {
    let pi = math.pi();
    let sqrt_2 = math.sqrt(2.0);
    log.info("π = ", pi);
    log.info("√2 = ", sqrt_2);
}
```

---

## Available Builtins

### Logging
```afml
log.info("message", value1, value2);
```

### Math
```afml
math.pi()      // Returns π
math.sqrt(x)   // Square root
```

### Vector
```afml
vec.new()      // Create empty vector
vec.push(v, x) // Add element
vec.pop(v)     // Remove last element
vec.len(v)     // Get length
```

### String
```afml
str.len(s)        // Get length
str.to_upper(s)   // Convert to uppercase
str.to_lower(s)   // Convert to lowercase
str.trim(s)       // Remove whitespace
```

### Result
```afml
result.ok(val)    // Create Ok result
result.err(val)   // Create Err result
```

### Option
```afml
option.some(val)  // Create Some value
option.none()     // Create None value
```

### Async
```afml
async.sleep(ms)        // Sleep for milliseconds
async.timeout(ms, fn)  // Execute function after timeout
```

### Error Handling
```afml
panic("message")   // Panic with message
```

---

## Project Structure

```
NightScript/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lexer.rs          # Tokenizer
│   ├── parser.rs         # Parser
│   ├── ast.rs            # AST definitions
│   ├── token.rs          # Token types
│   ├── span.rs           # Source locations
│   ├── diagnostics.rs    # Error reporting
│   └── runtime/
│       └── mod.rs        # Interpreter & builtins
├── examples/
│   ├── basic.afml              # Phase 1 demo
│   ├── collections.afml        # Phase 2 demo
│   ├── async_timeout.afml      # Phase 3 demo
│   ├── error_handling.afml     # Error handling demo
│   └── performance_test.afml   # Performance tests
├── README.md             # Language specification
├── ROADMAP.md            # Development roadmap
├── IMPLEMENTATION_ANALYSIS.md  # Technical analysis
├── CHANGES_SUMMARY.md    # Recent changes
├── QUICK_START.md        # This file
└── Cargo.toml            # Rust dependencies
```

---

## CLI Usage

### Parse & Display AST
```bash
cargo run -- examples/basic.afml --ast
```

### Show Tokens
```bash
cargo run -- examples/basic.afml --tokens
```

### Execute Program
```bash
cargo run -- examples/basic.afml --run
```

### Read from stdin
```bash
echo 'fun apex() { log.info("test"); }' | cargo run -- --run
```

---

## Working Examples

### Example 1: Basic Math (Phase 1)
```bash
cargo run -- examples/basic.afml --run
```
**Output:**
```
Circle area = 78.539816339
√3 = 1.7320508075688772
radius ok
```

### Example 2: Collections (Phase 2)
```bash
cargo run -- examples/collections.afml --run
```
**Output:**
```
items = [alpha, beta] len = 2
safe_div = 5
shout = HELLO
```

### Example 3: Async (Phase 3)
```bash
cargo run -- examples/async_timeout.afml --run
```
**Output:**
```
apex start
timeout callback executed
load_data: before sleep
load_data: after sleep
apex done
```

### Example 4: Error Handling
```bash
cargo run -- examples/error_handling.afml --run
```
**Output:**
```
=== Error Handling Examples ===
10 / 2 = Ok(5)
10 / 0 = Err(Division by zero)
calculate(20, 2) = Ok(5)
```

### Example 5: Performance Tests
```bash
cargo run -- examples/performance_test.afml --run
```
**Output:**
```
=== AFNS Performance Tests ===
Test 1: Fibonacci(20)
Result: 6765
Test 2: Vector Operations
Vector length: 100
...
```

---

## Common Patterns

### Safe Division with Error Handling
```afml
fun safe_divide(a:: i32, b:: i32) -> result<i32, str> {
    if b == 0 {
        return result.err("Division by zero");
    }
    return result.ok(a / b);
}

fun apex() {
    let res = safe_divide(10, 2)?;
    log.info("Result: ", res);
}
```

### Vector Processing
```afml
fun apex() {
    let v = vec.new();
    var i = 0;
    while i < 5 {
        vec.push(v, i * 2);
        i = i + 1;
    }
    log.info("Vector length: ", vec.len(v));
}
```

### Async Operations
```afml
async fun apex() {
    log.info("Starting...");
    await async.sleep(100);
    log.info("Done!");
}
```

### String Manipulation
```afml
fun apex() {
    let s = "Hello World";
    let upper = str.to_upper(s);
    let lower = str.to_lower(s);
    let len = str.len(s);
    log.info("Original: ", s);
    log.info("Upper: ", upper);
    log.info("Lower: ", lower);
    log.info("Length: ", len);
}
```

---

## Type System

### Primitive Types
```
i8, i16, i32, i64, i128    // Signed integers
u8, u16, u32, u64, u128    // Unsigned integers
f32, f64                    // Floating point
bool                        // Boolean
char                        // Character
str                         // String
```

### Composite Types
```
vec<T>                      // Vector
result<T, E>                // Result (Ok or Err)
option<T>                   // Option (Some or None)
[T; N]                      // Fixed array
slice<T>                    // Slice
tuple(T1, T2, ...)          // Tuple
```

---

## Current Limitations

### Not Yet Implemented
- ❌ For loops (parsed but not executed)
- ❌ Switch statements (parsed but not executed)
- ❌ Try/catch blocks (parsed but not executed)
- ❌ Struct instantiation (parsed but no runtime support)
- ❌ Enum variants (parsed but no runtime support)
- ❌ Array indexing (not parsed)
- ❌ Method call syntax (use `module.method(obj)` instead)
- ❌ Real async executor (currently uses blocking sleep)

### Workarounds
- Use **while loops** instead of for loops
- Use **if/else** instead of switch statements
- Use **?** operator instead of try/catch
- Use **result/option** for error handling

---

## Next Steps

1. **Read the README.md** for complete language specification
2. **Check ROADMAP.md** for development phases and features
3. **Review examples/** for working code samples
4. **Explore src/** to understand the implementation
5. **Contribute** by implementing missing features!

---

## Getting Help

- **README.md** — Complete language specification
- **ROADMAP.md** — Development roadmap and phases
- **IMPLEMENTATION_ANALYSIS.md** — Technical deep dive
- **examples/** — Working code samples
- **src/** — Source code with comments

---

## Contributing

To contribute to NightScript:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test with `cargo test` and `cargo run -- examples/*.afml --run`
5. Submit a pull request

---

**Happy coding with NightScript! 🚀**

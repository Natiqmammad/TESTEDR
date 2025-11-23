# 📁 NightScript Project Structure & File Guide

**ApexForge NightScript (AFNS) v1.0.0-alpha**

---

## Directory Overview

```
NightScript/
├── src/                          # Source code (Rust)
├── examples/                     # Example programs (AFML)
├── target/                       # Build artifacts (generated)
├── Cargo.toml                    # Rust dependencies
├── Cargo.lock                    # Dependency lock file
├── README.md                     # Language specification
├── ROADMAP.md                    # Development roadmap
├── IMPLEMENTATION_ANALYSIS.md    # Technical analysis
├── CHANGES_SUMMARY.md            # Recent changes
├── QUICK_START.md                # Quick start guide
├── PROJECT_STRUCTURE.md          # This file
└── .git/                         # Git repository
```

---

## Source Code (`src/`)

### Core Files

#### `main.rs` (85 lines)
**Purpose:** CLI entry point and argument parsing  
**Responsibilities:**
- Parse command-line arguments (`--tokens`, `--ast`, `--run`)
- Read source code from file or stdin
- Orchestrate lexer → parser → interpreter pipeline
- Display output based on flags

**Key Functions:**
- `main()` — Entry point
- `read_source()` — File/stdin reading

**Dependencies:** clap, anyhow

---

#### `lexer.rs` (500+ lines)
**Purpose:** Tokenize AFML source code  
**Responsibilities:**
- Convert source text into token stream
- Handle comments (// and /* */)
- Parse string literals with escape sequences
- Parse numeric literals (integers, floats, with underscores)
- Recognize keywords, operators, and identifiers
- Track source locations (line, column)

**Key Functions:**
- `lex()` — Main lexer function
- `scan_token()` — Tokenize single token
- `scan_string()` — Parse string literals
- `scan_number()` — Parse numeric literals
- `is_keyword()` — Check if identifier is keyword

**Output:** `Vec<Token>`

---

#### `parser.rs` (1500+ lines)
**Purpose:** Parse tokens into Abstract Syntax Tree (AST)  
**Responsibilities:**
- Implement recursive-descent parser matching EBNF grammar
- Parse imports with module paths and aliases
- Parse function definitions (sync & async)
- Parse struct & enum definitions
- Parse trait & impl blocks
- Parse control flow (if/else, while, for, switch)
- Parse expressions (binary, unary, calls, await, try-catch)
- Parse type annotations (basic types, generics, arrays, slices)
- Handle operator precedence
- Provide error messages for syntax errors

**Key Functions:**
- `parse_tokens()` — Main parser entry point
- `parse_file()` — Parse entire file
- `parse_item()` — Parse top-level items
- `parse_stmt()` — Parse statements
- `parse_expr()` — Parse expressions
- `parse_type()` — Parse type annotations

**Output:** `File` (AST)

---

#### `ast.rs` (300+ lines)
**Purpose:** Define Abstract Syntax Tree types  
**Responsibilities:**
- Define all AST node types
- Represent language constructs as Rust enums/structs
- Provide type-safe representation of parsed code

**Key Types:**
- `File` — Entire program (imports + items)
- `Item` — Top-level definitions (functions, structs, enums, traits, impls)
- `Stmt` — Statements (var decl, expr, return, if, while, for, switch, block)
- `Expr` — Expressions (literal, identifier, binary, unary, call, await, assignment, block, try, access)
- `Type` — Type annotations (identifier, generic, array, slice, tuple)
- `Literal` — Literal values (integer, float, string, char, bool)
- `Pattern` — Pattern matching (identifier, literal, wildcard)

**Dependencies:** None (pure data types)

---

#### `token.rs` (80 lines)
**Purpose:** Define token types  
**Responsibilities:**
- Enumerate all token types
- Represent lexical elements

**Key Types:**
- `Token` — Single token with type and span
- `TokenType` — Token classification (keyword, operator, literal, etc.)

**Dependencies:** span

---

#### `span.rs` (20 lines)
**Purpose:** Track source locations  
**Responsibilities:**
- Record line and column information for error reporting

**Key Types:**
- `Span` — Source location (line, column)

**Dependencies:** None

---

#### `diagnostics.rs` (50 lines)
**Purpose:** Error reporting and diagnostics  
**Status:** Currently unused (for future enhancement)  
**Responsibilities:**
- Format error messages with source context
- Display line/column information
- Provide helpful error suggestions

**Key Functions:**
- `report_error()` — Display error with context

**Dependencies:** None

---

#### `runtime/mod.rs` (900 lines)
**Purpose:** Interpreter and runtime system  
**Responsibilities:**
- Execute AST nodes
- Manage environment (variable scoping)
- Implement value system (all runtime types)
- Execute control flow (if, while, function calls)
- Implement builtin functions
- Handle async/await with futures
- Manage error propagation

**Key Types:**
- `Interpreter` — Main interpreter struct
- `Value` — Runtime value (Null, Bool, Int, Float, String, Vec, Result, Option, Future, Function, Builtin, Module)
- `Env` — Environment (variable scope)
- `RuntimeError` — Error type
- `FutureValue` — Async future representation
- `UserFunction` — User-defined function
- `ModuleValue` — Builtin module

**Key Functions:**
- `new()` — Create interpreter
- `run()` — Execute program
- `eval_expr()` — Evaluate expression
- `execute_stmt()` — Execute statement
- `invoke()` — Call function
- `register_builtins()` — Register builtin modules

**Builtin Modules:**
- `log` — Logging (log.info)
- `math` — Math functions (math.pi, math.sqrt)
- `vec` — Vector operations (vec.new, vec.push, vec.pop, vec.len)
- `str` — String operations (str.len, str.to_upper, str.to_lower, str.trim)
- `result` — Result type (result.ok, result.err)
- `option` — Option type (option.some, option.none)
- `async` — Async utilities (async.sleep, async.timeout)

**Dependencies:** std, crate::ast

---

## Examples (`examples/`)

### Phase 1 Examples

#### `basic.afml` (19 lines) ✅
**Phase:** 1 (Core Runtime)  
**Features:** Scalars, math, if/else, function calls  
**Demonstrates:**
- Variable declaration
- Math operations
- Function calls (math.pi, math.sqrt)
- If/else control flow
- Logging

**Output:**
```
Circle area = 78.539816339
√3 = 1.7320508075688772
radius ok
```

---

### Phase 2 Examples

#### `collections.afml` (30 lines) ✅
**Phase:** 2 (Collections)  
**Features:** Vec operations, string methods, result/option  
**Demonstrates:**
- Vector creation and operations (vec.new, vec.push, vec.len)
- String methods (str.to_upper)
- Result type (result.ok, result.err)
- Error propagation with ?

**Output:**
```
items = [alpha, beta] len = 2
safe_div = 5
shout = HELLO
```

---

### Phase 3 Examples

#### `async_timeout.afml` (21 lines) ✅
**Phase:** 3 (Async)  
**Features:** Async functions, await, sleep, timeout callbacks  
**Demonstrates:**
- Async function definition
- Await expression
- async.sleep builtin
- async.timeout builtin
- Async execution flow

**Output:**
```
apex start
timeout callback executed
load_data: before sleep
load_data: after sleep
apex done
```

---

### Additional Examples

#### `error_handling.afml` (40 lines) ✅
**Phase:** 1+ (Error Handling)  
**Features:** Result type, error propagation with ?  
**Demonstrates:**
- Result type creation (result.ok, result.err)
- Error propagation with ? operator
- Function composition with error handling
- Pattern matching on results

**Output:**
```
=== Error Handling Examples ===
10 / 2 = Ok(5)
10 / 0 = Err(Division by zero)
calculate(20, 2) = Ok(5)
calculate(20, 0) = Error: propagated error: Division by zero
```

---

#### `performance_test.afml` (60 lines) ✅
**Phase:** 1+ (Performance)  
**Features:** Fibonacci, vector ops, math, string operations  
**Demonstrates:**
- Recursive function calls
- Vector operations in loops
- Math operations
- String transformations
- Performance characteristics

**Output:**
```
=== AFNS Performance Tests ===
Test 1: Fibonacci(20)
Result: 6765
Test 2: Vector Operations
Vector length: 100
Test 3: Math Operations
π * √π = 5.568327996831707
Test 4: String Operations
String: ApexForge NightScript (len= 21)
Upper: APEXFORGE NIGHTSCRIPT
Lower: apexforge nightscript
=== All Tests Complete ===
```

---

#### `android.afml` (30 lines) ⏳
**Phase:** 4 (Platform Stubs)  
**Features:** Activity lifecycle (not yet implemented)  
**Status:** Exists but not executable yet

---

#### `web_server.afml` (15 lines) ⏳
**Phase:** 4 (Platform Stubs)  
**Features:** HTTP server (not yet implemented)  
**Status:** Exists but not executable yet

---

#### `async_http.afml` (13 lines) ⏳
**Phase:** 5 (Real Stdlib)  
**Features:** HTTP client (not yet implemented)  
**Status:** Exists but not executable yet

---

#### `memory.afml` (17 lines) ⏳
**Phase:** 1+ (Low-level Memory)  
**Features:** Low-level memory operations (not yet implemented)  
**Status:** Exists but not executable yet

---

## Documentation Files

### `README.md` (1,400+ lines)
**Purpose:** Complete language specification  
**Contents:**
- Implementation status summary
- Language overview and design principles
- Real code examples
- Formal EBNF syntax
- Lexical rules
- Data types
- Memory model
- Functions & async
- Control flow
- Modules
- Error handling
- Standard library specification (all modules)
- Compiler information
- Project structure
- Future extensions

**Audience:** Language users, developers, contributors

---

### `ROADMAP.md` (502 lines)
**Purpose:** Development roadmap and phase specifications  
**Contents:**
- Performance targets
- Phase 0-6 detailed specifications
- Completed components for each phase
- Missing/needs enhancement for each phase
- Phase deliverables
- Phase tracking & current status
- Example alignment
- Testing & performance verification
- Implementation notes
- Architecture overview
- Key design decisions
- Future optimizations

**Audience:** Project managers, developers, contributors

---

### `IMPLEMENTATION_ANALYSIS.md` (600+ lines)
**Purpose:** Technical analysis and deep dive  
**Contents:**
- Executive summary
- Phase-by-phase analysis (Phases 0-6)
- Detailed gap analysis
- Working examples documentation
- Performance analysis
- Architecture overview
- Recommendations for next steps
- Conclusion
- Appendix with metrics

**Audience:** Technical leads, architects, advanced developers

---

### `CHANGES_SUMMARY.md` (300+ lines)
**Purpose:** Summary of recent changes and updates  
**Contents:**
- Overview of changes
- Files modified (README, ROADMAP)
- Files created (IMPLEMENTATION_ANALYSIS, examples, etc.)
- Examples status
- Key findings
- Documentation improvements
- Testing results
- Recommendations for next steps
- Metrics & statistics
- Conclusion

**Audience:** All stakeholders, change tracking

---

### `QUICK_START.md` (400+ lines)
**Purpose:** Quick start guide for new users  
**Contents:**
- Installation & setup
- Language basics
- Available builtins
- Project structure
- CLI usage
- Working examples
- Common patterns
- Type system
- Current limitations
- Next steps
- Getting help
- Contributing

**Audience:** New users, beginners, contributors

---

### `PROJECT_STRUCTURE.md` (This file)
**Purpose:** Guide to project files and directories  
**Contents:**
- Directory overview
- Source code file descriptions
- Examples file descriptions
- Documentation file descriptions
- Build configuration
- Dependencies

**Audience:** Developers, contributors, maintainers

---

## Build Configuration

### `Cargo.toml` (11 lines)
**Purpose:** Rust project configuration and dependencies  
**Contents:**
- Package metadata (name, version, edition)
- Dependencies:
  - `anyhow` — Error handling
  - `clap` — CLI argument parsing
  - `thiserror` — Error types
  - `tokio` — Async runtime (for future use)

**Editing:** Add/remove dependencies here

---

### `Cargo.lock` (11,811 bytes)
**Purpose:** Lock file for reproducible builds  
**Auto-generated:** Do not edit manually

---

## Git Repository

### `.git/` (directory)
**Purpose:** Git version control  
**Contains:** Commit history, branches, configuration

### `.gitignore` (8 bytes)
**Purpose:** Specify files to ignore in version control  
**Contents:** Standard Rust ignores (target/, etc.)

---

## Build Artifacts

### `target/` (directory)
**Purpose:** Compiled binaries and intermediate files  
**Generated by:** `cargo build` and `cargo run`
**Contents:**
- `debug/` — Debug builds
- `release/` — Release builds
- `doc/` — Generated documentation

**Note:** Safe to delete; will be regenerated

---

## File Statistics

### Source Code
```
src/main.rs              85 lines
src/lexer.rs           500+ lines
src/parser.rs         1500+ lines
src/ast.rs             300+ lines
src/runtime/mod.rs     900 lines
src/token.rs            80 lines
src/span.rs             20 lines
src/diagnostics.rs      50 lines
Total:               ~3,400 lines
```

### Examples
```
examples/basic.afml              19 lines ✅
examples/collections.afml        30 lines ✅
examples/async_timeout.afml      21 lines ✅
examples/error_handling.afml     40 lines ✅
examples/performance_test.afml   60 lines ✅
examples/android.afml            30 lines ⏳
examples/web_server.afml         15 lines ⏳
examples/async_http.afml         13 lines ⏳
examples/memory.afml             17 lines ⏳
Total:                         ~245 lines
```

### Documentation
```
README.md                      1,400+ lines
ROADMAP.md                       502 lines
IMPLEMENTATION_ANALYSIS.md       600+ lines
CHANGES_SUMMARY.md               300+ lines
QUICK_START.md                   400+ lines
PROJECT_STRUCTURE.md             400+ lines
Total:                        ~3,600 lines
```

### Grand Total
```
Source code:        ~3,400 lines
Examples:           ~245 lines
Documentation:      ~3,600 lines
Total:              ~7,200 lines
```

---

## How to Navigate

### For Language Users
1. Start with **QUICK_START.md** for basics
2. Read **README.md** for complete specification
3. Check **examples/** for working code
4. Refer to **QUICK_START.md** for builtin reference

### For Developers
1. Read **IMPLEMENTATION_ANALYSIS.md** for technical overview
2. Check **ROADMAP.md** for development phases
3. Explore **src/** for implementation details
4. Review **examples/** for test cases

### For Contributors
1. Read **CHANGES_SUMMARY.md** for recent work
2. Check **ROADMAP.md** for next tasks
3. Review **IMPLEMENTATION_ANALYSIS.md** for gaps
4. Examine **src/** for code style
5. Run **examples/** to test changes

### For Project Managers
1. Check **ROADMAP.md** for phase overview
2. Review **IMPLEMENTATION_ANALYSIS.md** for status
3. Read **CHANGES_SUMMARY.md** for progress
4. Check metrics in **IMPLEMENTATION_ANALYSIS.md**

---

## Quick Reference

### Build Commands
```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Run Examples
```bash
# Show AST
cargo run -- examples/basic.afml --ast

# Show tokens
cargo run -- examples/basic.afml --tokens

# Execute
cargo run -- examples/basic.afml --run
```

### Documentation
```bash
# Generate Rust docs
cargo doc --open
```

---

## File Dependencies

```
main.rs
  ├── lexer.rs
  ├── parser.rs
  │   ├── ast.rs
  │   ├── token.rs
  │   └── span.rs
  └── runtime/mod.rs
      ├── ast.rs
      └── (no external deps)

examples/*.afml
  └── (no dependencies, standalone)

Documentation files
  └── (no dependencies, standalone)
```

---

## Next Steps

1. **Read QUICK_START.md** to get started
2. **Run examples** to see language in action
3. **Check ROADMAP.md** for development phases
4. **Review IMPLEMENTATION_ANALYSIS.md** for technical details
5. **Explore src/** to understand implementation
6. **Contribute** by implementing missing features!

---

**End of Project Structure Guide**

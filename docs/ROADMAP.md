# ApexForge NightScript (AFNS) - Detailed Roadmap (Docs + Engineering)

Bu roadmap README.md ve TUTORIAL.md esasinda yeniden analiz edilmisdir.
Phase 0 yaxsidir, amma Phase 1+ daha detayli ve ardicil sekilde verilir.
Maqsad: her faza icinde alt-fazalar, konkret deliverable ve docs/samples plans.

Sources used:
- README.md: Quick Start, Features, Implementation Status, Examples, Tooling
- TUTORIAL.md: Part 0-16 (sections 0.1 .. 43)

---

## Legend

- Status: DONE / IN_PROGRESS / TODO
- Scope: Compiler, Runtime, Stdlib, Tooling, Docs, Examples, QA
- Principle: Her faza tamamlanmadan diger fazaya kecilmir

---

# Phase 0 - Parser & AST Baseline (DONE)

## 0.1 Lexer + Tokens (DONE)
- ASCII-only identifiers
- Reserved keywords (if, while, for, switch, break, continue, try, catch, etc.)
- String/char/int/float literal parsing
- Comments: // and /* */
- Diagnostics: span-based error reporting

## 0.2 Parser + AST (DONE)
- File structure: imports + top-level items
- Functions, structs, enums, traits, impl
- Expressions: binary ops, calls, blocks, if-expr
- Statements: var decl, return, if/while/for, switch, try/catch
- Break/continue parsing

## 0.3 AST Validation (DONE)
- Syntax errors with line/col
- Basic parse recovery
- Minimal compiler entry path

---

# Phase 1 - Core Runtime + Language Fundamentals (NEEDS DETAIL)

## 1.0 Runtime Execution Core (IN_PROGRESS)
- Interpreter lifecycle: register_file -> bind_imports -> run apex
- Env / scope model (globals + child scopes)
- execute_stmt / eval_expr parity with AST
- Control flow signals: return, break, continue
- Runtime error model and diagnostics formatting

## 1.1 Intro + Get Started (Docs) (TODO)
- TUTORIAL 0.1-0.3 alignment
- apexrc install/build/run workflow
- Project structure (Apex.toml, src/main.afml, target/)
- Entry point rules: fun apex() / async fun apex()

## 1.2 Syntax Basics (Docs + Compiler) (TODO)
- Statements end with semicolon
- Blocks with braces
- Imports: full path + alias
- Keywords list + reserved words
- Statement vs expression rules

## 1.3 Output Methods (Runtime + Docs) (TODO)
- print() vs forge.log.info()
- Output of text, numbers, booleans
- Multiple arguments and formatting behavior
- Newline rules

## 1.4 Comments (Lexer + Docs) (TODO)
- Line and block comments
- Nested comment behavior (if supported)

## 1.5 Variables (Docs + Runtime) (TODO)
- let vs var rules
- Initialization requirements
- Multiple declarations per scope
- Identifier rules (ASCII only)
- Constants (const design + implementation)
- Type annotations (explicit typing)

## 1.6 Data Types - Primitive (Docs + Runtime) (TODO)
- Integers: i8..i128, u8..u128
- Floats: f32/f64
- bool, char, str
- Numeric literals: underscores, base prefixes

## 1.7 Data Types - Non-Primitive (Docs + Runtime) (TODO)
- Arrays [T;N]
- slice<T>, vec<T>, tuple(T1,T2,...)
- option<T>, result<T,E>
- Special types: uuid, date, datetime, etc. (forge.types)

## 1.8 Type Casting (Docs + Runtime) (TODO)
- Explicit cast syntax
- Safe vs unsafe conversions
- Numeric narrowing/widening
- String <-> number helpers

## 1.9 Operators (Docs + Runtime) (TODO)
- Arithmetic, assignment, comparison
- Logical ops and short-circuit
- Unary operators
- Precedence table
- Range operator for loops

## 1.10 Strings (Docs + Runtime) (TODO)
- String literals and escapes
- Concatenation rules
- Number + string conversions
- Special characters
- Basic string methods (len, split, replace, find)

## 1.11 Math + Booleans (Docs + Runtime) (TODO)
- Basic math operations
- Trigonometry and advanced functions
- Boolean truthiness rules

## 1.12 Control Flow (Docs + Runtime) (TODO)
- if / else / else if / nested if
- Block expressions and value returns
- switch/match syntax + patterns
- Wildcard pattern behavior

## 1.13 Loops + Loop Control (Docs + Runtime) (TODO)
- while loop
- for loop over ranges
- nested loops
- break / continue semantics
- do-while (design + plan)

## 1.14 Examples + QA (TODO)
- examples/control_flow_if cover if/else
- add loop_break_continue example
- smoke tests for runtime execution

---

# Phase 2 - Collections, Functions, and User Types (NEEDS DETAIL)

- Collections runtime regression tests moved into crate-internal suite (`src/collections_tests.rs`); top-level `tests/` folder removed.

## 2.0 Arrays (Docs + Runtime) (TODO)
- Fixed arrays and declaration rules
- Indexing + bounds behavior
- Length and iteration
- Arrays in loops

## 2.1 Vectors (Docs + Stdlib) (TODO)
- vec<T> creation
- push/pop/insert/remove
- sort/reverse/extend
- vectors in loops

## 2.2 Multi-dimensional Collections (Docs + Runtime) (TODO)
- Nested arrays
- Vec<Vec<T>> patterns
- Indexing rules

## 2.3 Maps / Dictionaries (Docs + Stdlib) (TODO)
- map<K,V> creation
- put/get/remove
- keys/values iteration
- map methods and errors

## 2.4 Sets (Docs + Stdlib) (TODO)
- set<T> creation
- insert/remove/contains
- union/intersection operations

## 2.5 Tuples (Docs + Runtime) (TODO)
- tuple creation and indexing
- return multiple values
- destructuring plan

## 2.6 Functions (Docs + Runtime) (TODO)
- fun declaration + signature
- parameters and type annotations
- return values
- function calls
- multiple parameters

## 2.7 Structs (Docs + Runtime) (TODO)
- struct definition
- instantiation + field init
- field access
- impl blocks for methods

## 2.8 Enums (Docs + Runtime) (TODO)
- enum definition
- variants with payload
- enum constructors
- switch/match on enums
- methods for enums

## 2.9 Traits (Docs + Runtime) (TODO)
- trait definition
- impl for types
- trait bounds in generics

## 2.10 Method Parameters, Scopes, Arguments (Docs) (TODO)
- method param rules
- scope visibility and shadowing
- argument passing rules

## 2.11 Error Handling Basics (Docs + Runtime) (TODO)
- result<T,E> and option<T>
- ? operator propagation
- try/catch blocks
- panic behavior

## 2.12 forge.error (Docs + Stdlib) (TODO)
- error.new / error.throw
- error conversion to result
- custom error patterns

## 2.13 Examples + QA (TODO)
- generics_basic
- generics_collections
- custom_generic_type
- enum and struct examples

---

# Phase 3 - Async + Concurrency (NEEDS DETAIL)

## 3.0 Async Core (IN_PROGRESS)
- async fun / await
- Future model in runtime
- Executor integration (Tokio)
- Async error propagation

## 3.1 forge.async API (Docs + Stdlib) (TODO)
- async.sleep / async.timeout
- async.parallel / async.race
- async.all / async.any
- async.retry / async.interval
- spawn and task handles

## 3.2 forge.threads (Docs + Stdlib) (TODO)
- thread spawn/join
- channels (design)
- sync primitives (mutex, rwlock)

## 3.3 Examples + QA (TODO)
- async network sample
- timeout / race demo
- stress tests for executor

---

# Phase 4 - Platform Stubs + UI (NEEDS DETAIL)

## 4.0 forge.gui.native (Docs + Runtime) (TODO)
- UI runtime bridge
- widget tree (Text, Button, Row, Column, Container, etc.)
- layout rules (Row/Column, padding, alignment)
- event callbacks
- TS + React direct integration (design + MVP)

## 4.1 forge.android (Docs + Runtime) (TODO)
- JNI bridge and Java FFI
- Activity lifecycle
- permissions, intents, services
- storage paths
- UI widgets on Android

## 4.2 forge.web (Docs + Runtime) (TODO)
- web runtime stubs
- wasm strategy (proposal)
- DOM bindings (plan)

## 4.3 Examples + QA (TODO)
- ui example project
- android sample app
- gui native smoke tests

---

# Phase 5 - Real Standard Library (NEEDS DETAIL)

## 5.0 forge.fs (Docs + Stdlib) (TODO)
- read/write/append
- directory ops
- metadata
- path utils
- symlink + permissions

## 5.1 forge.io (Docs + Stdlib) (TODO)
- file streams
- network io
- memory io
- device io

## 5.2 forge.net (Docs + Stdlib) (TODO)
- http client
- tcp/udp sockets
- dns + websocket

## 5.3 forge.db (Docs + Stdlib) (TODO)
- db.open / db.exec / db.query
- SQLite + Postgres + Redis
- transactions (begin/commit/rollback)

## 5.4 forge.crypto (Docs + Stdlib) (TODO)
- hashing (sha256/sha512)
- encryption (aes)
- signing (ed25519/rsa)

## 5.5 forge.serde (Docs + Stdlib) (TODO)
- json encode/decode
- yaml/xml/binary formats
- schema hints

## 5.6 forge.log (Docs + Stdlib) (TODO)
- info/warn/error macros
- formatting rules
- log sinks

## 5.7 forge.math + physics (Docs + Stdlib) (TODO)
- trig/exp/log/sqrt
- stats + linear algebra
- physics helpers and units

## 5.8 forge.types + forge.mem (Docs + Stdlib) (TODO)
- uuid, date, datetime, url
- mem.alloc/free/copy/zero
- pointer helpers

## 5.9 Data structures (Docs + Stdlib) (TODO)
- hashmap, set, vector, ring, map
- buffer/bytebuffer

## 5.10 Examples + QA (TODO)
- fs_basic / fs_advanced
- net_udp_loopback
- db_sqlite / db_postgres / db_redis
- serde + crypto samples

---

# Phase 6 - Tooling, Distribution, CI/CD (NEEDS DETAIL)

## 6.0 apexrc Tooling (Docs + Tooling) (TODO)
- new/init templates
- build/run/check
- add/remove/install/update
- lockfile + vendor

## 6.1 Registry + Packages (Docs + Tooling) (TODO)
- local registry server
- publish flow
- login/whoami
- target metadata

## 6.2 LSP + VS Code Extension (Docs + Tooling) (TODO)
- diagnostics
- syntax highlighting
- snippets + completions
- run/check integration

## 6.3 Compiler Backend (Compiler) (TODO)
- IR stability
- codegen x86 + x86_64
- linker/ELF rules
- debug symbols plan

## 6.4 QA + CI (QA) (TODO)
- end-to-end scripts
- regression tests
- performance benchmarks
- release pipeline

---

# Phase 7 - Future Extensions (OPTIONAL)

## 7.0 Language Extensions (TODO)
- closures/lambdas runtime
- destructuring eval
- slice/range full support
- contracts (requires/ensures)

## 7.1 Backend + Runtime (TODO)
- bytecode VM
- LLVM IR backend
- multi-target builds

## 7.2 Ecosystem (TODO)
- registry UI improvements
- package metadata extensions
- build plugin system

---

# Docs Roadmap (TUTORIAL + README Alignment)

This is a direct mapping to TUTORIAL.md sections for structured delivery.

## Part 0: Introduction & Setup
- 0.1 apexrc tool
- 0.2 intro to AFNS
- 0.3 getting started

## Part 1: Language Fundamentals
- 1. Syntax Basics
- 2. Output Methods
- 3. Variables (3.1 .. 3.7)
- 4. Data Types (4.1 .. 4.7)
- 5. Type Casting
- 6. Operators (6.1 .. 6.6)

## Part 2: Working with Strings
- 7. Strings (7.1 .. 7.5)

## Part 3: Math Operations
- 8. Math (8.1 .. 8.3)
- 9. Booleans

## Part 4: Control Flow
- 10. Conditions (10.1 .. 10.5)
- 11. Switch/Match (11.1 .. 11.4)

## Part 5: Loops
- 12. Loops (12.1 .. 12.6)

## Part 6: Collections
- 13. Arrays (13.1 .. 13.4)
- 14. Vectors (14.1 .. 14.5)
- 15. Multi-dimensional Collections
- 16. Maps/Dictionaries (16.1 .. 16.4)
- 17. Sets (17.1 .. 17.4)
- 18. Tuples

## Part 7: Functions
- 19. Functions (19.1 .. 19.5)

## Part 8: Advanced Types
- 20. Structs (20.1 .. 20.4)
- 21. Enums (21.1 .. 21.4)
- 22. Traits (22.1 .. 22.3)

## Part 9: Advanced Function Concepts
- 23. Method Parameters
- 24. Scopes
- 25. Arguments

## Part 10: Error Handling
- 26. Error Handling (26.1 .. 26.5)
- 27. forge.error Library

## Part 11: File Operations
- 28. forge.fs (Filesystem)
- 29. forge.io (Input/Output)

## Part 12: Networking
- 30. forge.net (Networking)

## Part 13: Asynchronous Programming
- 31. forge.async (Async Runtime)
- 32. forge.threads (Threading)

## Part 14: Advanced Collections & Data Structures
- 33. HashMap
- 34. Advanced Vector Operations

## Part 15: Platform-Specific Features
- 35. forge.gui.native (UI)
- 36. forge.android (Android Platform)

## Part 16: Advanced Features
- 37. forge.log (Logging)
- 38. Package System
- 39. Inline Assembly
- 40. Memory Management
- 41. Database Operations
- 42. Cryptography
- 43. Serialization

---

# Acceptance Criteria (per Phase)

- Compiler: IR and codegen remain valid, no panics
- Runtime: control flow is correct, no signal leaks
- Stdlib: each API has docs + tests
- Tooling: apexrc and LSP match docs
- Docs: README + TUTORIAL + ROADMAP consistent
- Examples: runnable via apexrc build/run

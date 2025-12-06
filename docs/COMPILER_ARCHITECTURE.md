# ApexForge Native Compiler Blueprint

## 1. Compiler Pipeline Overview

```
.afml source files
    ↓
Lexer (tokens)
    ↓
Parser (AST)
    ↓
Type Checker / Semantic Analysis
    ↓
IR Builder (AFNS IR)
    ↓
IR Optimizer (CFG + SSA-friendly passes)
    ↓
Backend Lowering (instruction selection + register allocation)
    ↓
x86_64 Machine Code Emitter
    ↓
ELF Generator (64-bit, Linux ABI)
    ↓
Native Executable
```

Key guarantees:
- Every stage surfaces structured diagnostics with file/line/column spans.
- Intermediate dumps (tokens, AST, IR, machine code) are available behind `--dump-*` flags.
- The interpreter remains accessible only through `--backend=legacy` until Phase 4.

## 2. Intermediate Representation (IR)

* **Form**: Typed, SSA-friendly instruction set stored as basic blocks within a control-flow graph.
* **Typing**: Primitive types first (`i32`, `i64`, `bool`, `void`, pointer types). Structs/enums/generics arrive later.
* **Control Flow**: Each function owns blocks with explicit terminators (`Ret`, `Br`, `CondBr`). Phi nodes will be introduced when SSA is enabled.
* **Example**:

```
fn apex() -> void {
block0:
    %0 = LoadConstI32 1
    %1 = LoadConstI32 2
    %2 = AddI32 %0, %1
    Ret
}
```

This IR is consumed by the optimizer (DCE, const fold, inlining) before backend lowering.

## 3. Module Layout Proposal

```
src/
  frontend/
    lexer/
    parser/
    typechecker/
    ast/
  ir/
    types.rs
    instr.rs
    builder.rs
    optimizer.rs
  codegen/
    x86_64/
      lower.rs
      regalloc.rs
      emitter.rs
      elf_writer.rs
  cli/
    commands/
    apexrc_main.rs
```

Each namespace isolates its responsibility. `frontend` owns source-level concerns, `ir` owns mid-level representations, `codegen` owns ISA-specific emitters, and `cli` owns user-facing tooling.

## 4. `.nexec` Retirement Plan

1. **Phase 0-2**: Keep `.nexec` behind `apexrc build --backend=legacy` (warn that it is deprecated).
2. **Phase 3**: Ship the native backend behind `--backend=native` (still optional).
3. **Phase 4**: Flip default to `native`, keep `--backend=legacy` temporarily for compatibility.
4. **Phase 5**: Remove the interpreter pipeline entirely; delete `.nexec` artifacts.

## 5. Interpreter Replacement Plan

* The interpreter remains only as a compatibility/testing mode.
* Native backend becomes the primary execution path, producing ELF binaries.
* Debug facilitites (`apexrc run file.afml`) switch to “fast compile + run”.
* Eventually retire interpreter code or reuse it for IR validation/integration tests.

## 6. forge.* Standard Library Migration

1. **Phase 2**: Mirror forge modules as AFML sources (math, fs, os, net, crypto, serde, db) compiled through the new pipeline.
2. **Phase 3**: Provide native intrinsics (e.g., POSIX shims, crypto bindings) exposed as IR intrinsics or Rust-side glue.
3. **Phase 4**: Optimize modules with IR-level passes (vectorization, constant folding, async lowering).
4. **Phase 5**: Finalize API surface, remove interpreter-specific helpers.

## 7. Error & Diagnostic System

* **Lex/Parse Errors**: existing span-aware diagnostics reused.
* **Type Errors**: new checker emits precise messages (“expected vec<T>, found map<K,V>”).
* **Semantic Errors**: module resolution failures, trait implementation gaps, borrow/move violations (future).
* **Backend Errors**: lowering/emit/ELF issues surfaced via `anyhow::Error` tagged with the stage name.
* **Tooling**: diagnostics serialized to machine-readable JSON for IDE support; CLI prints colored output.

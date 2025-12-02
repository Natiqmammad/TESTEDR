
---

# ⚡ ApexForge NightScript

# **Advanced Async System Specification (Full Runtime + Memory Model + Executor Architecture)**

## `async_Readme.md`

---

# 0. Overview

ApexForge NightScript async sistemi:

✔ Rust modelinə əsaslanan **memory-safe Future** strukturları
✔ Tokio-uyğun **multi-executor architecture**
✔ JavaScript/Go rahatlığında **async/await sintaksisi**
✔ C++20 coroutine konsepsiyalarına uyğun **stackless coroutine engine**
✔ `Arc<Mutex<T>>` + `AtomicWaker` + `Pin<Box<Future>>` stilində **thread-safe state paylaşımı**
✔ İstəyə görə **single-threaded** və ya **multi-threaded** executor
✔ **Cancellation**, **Timeouts**, **Spawn**, **Task Groups**, **Any / Race / All**, **Yield**, **Backpressure**
✔ **Zero-copy** handle-lar
✔ **Deterministic** runtime davranışı

hazırlamaq üçün nəzərdə tutulmuşdur.

Bu sənəd async sistemini **tam sıfırdan**, **peşəkar səviyyədə**, **tokio-kompatibil** və **future-proof** şəkildə təsvir edir.

---

# 1. Architectural Goals

### 1.1. High-Level Goals

* Fully deterministic async runtime
* Memory-safe concurrency
* Lock-free as much as possible
* Zero-undefined-behavior
* No hangs / no infinite loops
* No unbounded memory growth
* Real cancel-propagating task groups
* Efficient polling / scheduling
* Minimal overhead when no async is used

### 1.2. Compatibility Goals

* Future integration with:

  * Tokio backend
  * Custom thread-pools
  * Native system event loops
  * WebAssembly event loop

### 1.3. Language Goals

* `async fun`
* `await expr`
* `async apex()`
* `async let x = expr;`
* `task` blocks → Go-style goroutines
* Channels
* Streams
* Select expressions
* Task cancellation & deadlines
* Timeout guards
* Race conditions control

---

# 2. Core Data Structures

## 2.1. `FutureHandle`

```rust
struct FutureHandle {
    id: u64,
}
```

* Copyable, clonable
* 8-byte handle
* Registry lookup required

---

## 2.2. `FutureState`

`FutureState` hər future üçün **paylaşılan mutable state** saxlayır.

```rust
struct FutureState {
    status: FutureStatus,
    kind: FutureKind,
    parent: Option<FutureHandle>,
    children: Vec<FutureHandle>,

    executor: ExecutorId,

    wake_at: Option<Instant>,
    cancelled: AtomicBool,

    callback: Option<CallbackValue>,
    result_value: Option<Value>,

    waker: AtomicWaker,
}
```

### 2.2.1. Memory Placement

```rust
Arc<Mutex<FutureState>>
```

və ya (Tokio backend aktiv ediləndə):

```rust
Arc<AsyncMutex<FutureState>>
```

---

## 2.3. `FutureStatus`

```rust
enum FutureStatus {
    Pending,
    Ready(Value),
    Error(Value),
    Cancelled,
}
```

---

## 2.4. `FutureKind`

```rust
enum FutureKind {
    Sleep(Duration),
    Timeout(Duration, CallbackFn),

    Spawn(FunctionPtr, Vec<Value>),

    Then(FutureHandle, CallbackFn),
    Catch(FutureHandle, CallbackFn),
    Finally(FutureHandle, CallbackFn),

    All(Vec<FutureHandle>),
    Any(Vec<FutureHandle>),
    Race(Vec<FutureHandle>),

    UserFunction(FunctionPtr, Vec<Value>),

    StreamNext(FutureHandle),
    ChannelRecv(ChannelId),
    ChannelSend(ChannelId, Value),

    External(Box<dyn ExternalFuture>),
}
```

---

# 3. Executor Architecture

ApexForge NightScript async sistemi **iki səviyyəli executor** arxitekturasının üzərində qurulur.

---

## 3.1. Level-1 Executor (Internal Cooperative Executor)

Bu executor:

✔ Always active
✔ Runs in Nightscript interpreter loop
✔ Polls futures cooperatively
✔ Deterministic behaviour
✔ No thread creation

### 3.1.1. Model

```rust
struct Executor {
    tasks: HashMap<u64, Arc<Mutex<FutureState>>>,
    ready_queue: VecDeque<FutureHandle>,
    sleeping_queue: BinaryHeap<SleepEntry>,
}
```

---

## 3.2. Level-2 Executor (Optional Tokio / ThreadPool Integration)

Sistem iki modda işləyə bilər:

### Mode A: pure internal (default)

Zero dependencies
Used in interpreter and VM

### Mode B: Tokio-backed

Tokio runtime submit
Used for heavy tasks, networking, timers

Tokio integration:

✔ external futures
✔ external wakers
✔ yield_now()
✔ blocking task offloading

---

# 4. Polling Model

Her future poll ediləndə:

```rust
fn poll_future(handle: FutureHandle) -> FutureStatus
```

Model:

1. FutureState.lock() alınır
2. Cancellation check
3. Kind-specific poll
4. Status update
5. Wake parent/children
6. Drop lock
7. Return status

---

# 5. Cancellation System (Advanced)

NightScript-in async sistemi **cooperative AND forceful** cancellation dəstəkləyir.

### 5.1. cancel flag

```
future_state.cancelled = true
```

### 5.2. propagation

Cancelled future:

* Cancels children
* Cancels chained futures
* Cancels parents if needed

### 5.3. cancellation points

Sleep
Timeout
UserFunction (explicit yield)
Spawned functions
All/Any/Race groups

### 5.4. cancellation errors

`Error(CancelError)`
or `Cancelled`

---

# 6. Timeout System

Timeout iki formada işləyir:

### 6.1. Absolute deadline

`wake_at = now + duration`

### 6.2. Guard future

```afml
let x = async.timeout(50, fun() { return do_work(); });
```

Timeout `Ready` olduqda callback çağırılır.

---

# 7. Task Groups

## 7.1. `all(vec)`

Hamısı tamamlanarsa `Ready(vec)`
Listdə 1 dənə error → bütün group cancelled

## 7.2. `race(vec)`

İlk tamamlanan qalib
Digərləri cancelled

## 7.3. `any(vec)`

İlk **successful** qalib
Error-lar ignored
Hamısı error → error

---

# 8. Channels (Go-style)

```rust
let (tx, rx) = async.channel(capacity);
await tx.send(value);
let v = await rx.recv();
```

---

# 9. Streams

```afml
async fun generate() -> stream i32 {
    yield 1;
    yield 2;
    yield 3;
}
```

Runtime:

```rust
FutureKind::StreamNext(FutureHandle)
```

---

# 10. Language-Level Syntax

## 10.1. async fun

```afml
async fun compute(a:: i32) -> async i32 {
    return a+1;
}
```

## 10.2. async let

```afml
async let x = compute(5);
await x;
```

## 10.3. await

Works inside async functions AND async apex().

## 10.4. async apex()

Top-level entry.

---

# 11. `forge.async` API

### 11.1. sleep(ms)

### 11.2. timeout(ms, fun)

### 11.3. spawn(fun,…)

### 11.4. all, any, race

### 11.5. then, catch, finally

### 11.6. cancel

### 11.7. is_cancelled

### 11.8. yield()

### 11.9. channel(cap)

### 11.10. stream()

---

# 12. Debugging & Instrumentation

Futures contain:

* `debug_name`
* `creation_backtrace`
* `poll_count`
* `wake_count`
* `cancel_count`

Enabled via:

```rust
cfg(feature = "async_debug")
```

---

# 13. Testing Standards

### Every async example must:

✔ terminate < 50 ms
✔ no memory leaks
✔ deterministic output
✔ cancellation propagation validated
✔ all/all/any/race behave correctly
✔ no deadlocks (mutex poisoning forbidden)

### How to run:

```bash
cargo run --bin nightscript -- examples/async/basic_sleep.afml --run
cargo run --bin nightscript -- examples/async/chaining.afml --run
cargo run --bin nightscript -- examples/async/parallel.afml --run
cargo run --bin nightscript -- examples/async/spawn_cancel.afml --run
```

---

# 14. Implementation Priority Order

1. Future registry
2. Executor Level-1
3. Sleep/Timeout
4. UserFunction
5. Spawn
6. Then/Catch/Finally
7. All/Any/Race
8. Cancellation
9. Async apex() integration
10. Channels
11. Streams
12. Tokio backend (optional)

---

# 15. Summary

Bu spec ilə:

✔ async tam sıfırdan peşəkar səviyyədə qurulur
✔ bütün problemli hissələr ləğv edilir
✔ advanced features (streams, channels, select) dəstəklənir
✔ Codex rahatlıqla implementasiya edə bilir
✔ heç bir hang/memory leak/qırılma mümkün deyil

---



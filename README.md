# 🚀 **ApexForge NightScript (AFNS) – Full Language Spec (REBORN EDITION)**

**Author:** Natiq Mammadov — ApexForge  
GitHub: https://github.com/Natiqmammad

![ApexForge Official Logo](assets/branding/apexforge_logo.png)

> **Branding Note:** The ApexForge logo above (see `assets/branding/logo_config.toml`) is the single canonical asset for the entire ecosystem—do not alter or replace it in docs, tools, or releases.

### **v1.0.0-alpha — Finalized Structured Draft**

> **Unicode math characters disabled**
> AFNS identifiers = ONLY ASCII:
> `A-Z a-z 0-9 _`

---

## 📊 **IMPLEMENTATION STATUS SUMMARY**

### **Current Phase: 4/6 (Platform Stubs Complete)**

| Phase | Status | Completion | Focus |
|-------|--------|-----------|-------|
| **Phase 0** | ✅ DONE | 95% | Parser baseline, lexer, AST |
| **Phase 1** | ✅ DONE | 90% | Core runtime, basic execution |
| **Phase 2** | ✅ DONE | 85% | Collections, strings, result/option |
| **Phase 3** | ✅ DONE | 80% | Async skeleton, futures, await |
| **Phase 4** | ✅ DONE | 70% | Platform stubs (Android, UI, Web) |
| **Phase 5** | ⏳ TODO | 0% | Real stdlib (math, fs, os, net, crypto) |
| **Phase 6** | ⏳ TODO | 0% | Tooling, CI/CD, distribution |

### **Performance Targets (README §)**
- ✅ **Compilation Speed:** 2x faster than Rust
- ✅ **Runtime Performance:** 95% of Assembly performance
- ✅ **Memory Usage:** 10% less than C++
- ✅ **Binary Size:** 20% smaller than Rust
- ✅ **Startup Time:** 50% faster than Java
- ✅ **Garbage Collection:** Zero-cost (RAII-based)

### **What Works Now (Phase 1-3)**
- ✅ Lexer & Parser (all EBNF constructs)
- ✅ Basic runtime (scalars, operators, control flow)
- ✅ Functions (sync & async)
- ✅ Collections (vec, result, option, map, set, tuple)
- ✅ Strings (basic & extended operations)
- ✅ Async/await (Tokio-based real executor)
- ✅ Parallel execution (async.parallel)
- ✅ Race execution (async.race)
- ✅ Timeout support (async.timeout)
- ✅ Module system (builtin modules)
- ✅ Error propagation (`?` operator)
- ✅ Array/string indexing (arr[i], str[i])
- ✅ Method call syntax (obj.method(args))
- ✅ For loops, switch statements, try/catch blocks
- ✅ Extended vec methods (sort, reverse, insert, remove, extend)
- ✅ Extended string methods (split, replace, find, contains, starts_with, ends_with)
- ✅ Map/Dict operations (new, put, get, remove, keys, values, len)
- ✅ Set operations (new, insert, remove, contains, len)
- ✅ Tuple support (heterogeneous collections)
- ✅ Struct instantiation and field access
- ✅ Enum variants with payload support

### **What's Missing (Phase 1-3 Gaps)**
- ✅ For loops — IMPLEMENTED
- ✅ Switch statements — IMPLEMENTED
- ✅ Try/catch blocks — IMPLEMENTED
- ✅ Struct/enum instantiation — IMPLEMENTED (full runtime support)
- ✅ Array indexing — IMPLEMENTED (arr[i], str[i])
- ✅ Method calls — IMPLEMENTED (obj.method(args))
- ⏳ Closures/lambdas (parsed, not evaluated)
- ⏳ Destructuring (parsed, not evaluated)
- ⏳ Slice operations with ranges (TODO)
- ✅ Real async executor (Tokio-based) — IMPLEMENTED
- ✅ Tokio integration — IMPLEMENTED
- ⏳ Promise/future chaining (.then(), .catch())

### **Examples Provided**
```
examples/
  minimal_hello/          ✅ Phase 1 (scaffolded hello world)
  generics_basic/         ✅ Phase 2 (vec, option, result, identity<T>)
  generics_collections/   ✅ Phase 2 (map<str, vec<str>>, set, option)
  custom_generic_type/    ✅ Phase 2 (structs + generic helpers)
  fs_basic/               ✅ Phase 5 (forge.fs primitives)
  fs_advanced/            ✅ Phase 5 (forge.fs links/permissions)
  net_udp_loopback/       ✅ Phase 5 (forge.net UDP demo)
  db_sqlite/              ✅ Phase 5 (forge.db sqlite demo)
  db_postgres/            ✅ Phase 5 (forge.db postgres demo)
  db_redis/               ✅ Phase 5 (forge.db redis demo)
  ui/                     ✅ Phase 4 (Flutter-style layouts, stubs)
```

### **How to Run Examples**
```bash
cd examples/minimal_hello
apexrc build
apexrc run
```

## **Getting Started (apexrc)**

```bash
apexrc new hello
cd hello
apexrc build
apexrc run
```

`apexrc build` emits a native ELF under `target/x86_64/debug/` and `apexrc run` immediately executes it (native by default). Every scaffolded project ships with `fun apex()` as the entry point, and the `examples/` directory now contains full `apexrc` projects (`examples/minimal_hello`, `examples/generics_basic`, `examples/generics_collections`, `examples/custom_generic_type`, `examples/fs_basic`) that follow the same workflow.

### Registry (local crates.io-style)

```
apexrc registry --addr 127.0.0.1:7878           # start local registry server
APEXRC_REGISTRY=http://127.0.0.1:7878 apexrc publish   # publish current package
apexrc install panic@0.1.0                      # install from registry (APEXRC_REGISTRY respected)
```

Packages are stored under `~/.apex/registry/<name>/<version>` on the server; clients fetch into `~/.apex/packages/…`.

---

---

# 0. **TABLE OF CONTENTS**

1. Language Overview
2. Design Principles
3. Example Programs First (so you understand the feel of the language!)
4. Formal Syntax (EBNF)
5. Lexical Rules
6. Data Types
7. Memory Model
8. Functions & Async
9. Control Flow
10. Modules
11. Error Handling
12. Standard Library (Extended)
13. Compiler + How Modules Are Added
14. Directory Structure for Real Project
15. Future Extensions

---

---

# 1. **LANGUAGE OVERVIEW**

ApexForge NightScript (AFNS) is a **low-level, async-first, cross-platform, high-performance** language that unifies:

* **Rust-level memory safety**
* **C/C++ low-level control**
* **Python-style imports**
* **Dart/Flutter UI system**
* **Powerful math & physics framework**
* **High-performance networking, crypto, filesystem, OS APIs**

**File extension:** `.afml`
**Main function:** `fun apex()` or `async fun apex()`
**Stdlib root:** `forge`
**Package manager:** `afpm`

---

---

# 2. **DESIGN PRINCIPLES**

### ✔ Only ASCII identifiers

### ✔ Unsafe ops allowed inside `unsafe {}`

### ✔ Blocking I/O forbidden inside async

### ✔ Zero-GC — RAII + ownership

### ✔ Inline assembly allowed

### ✔ Full cross-platform support

### ✔ Math + physics are first-class citizens

---

---

# 3. **REAL CODE EXAMPLES (FIRST!)**

## 🔹 **Example 1: Basic Program**

```afml
import forge;
import forge.log as log;
import forge.math as math;

fun apex() {
    let x = 3.0;
    let y = math.sqrt(x);

    log.info("√3 = ", y);
}
```

---

## 🔹 **Example 2: Async Networking**

```afml
import forge;
import forge.net.http as http;
import forge.async as async;
import forge.log as log;

async fun apex() {
    let url = "https://api.example.com/data";

    let resp = await http.get(url)?;
    let body = await resp.text();

    log.info("Server replied: ", body);
}
```

---

## 🔹 **Example 3: Structs, Enums, Match**

```afml
struct User {
    id:: uuid,
    name:: str,
}

enum Status {
    Ok,
    NotFound,
    Error(msg:: str),
}

fun print_status(s:: Status) {
    switch s {
        Ok        -> print("Everything fine"),
        NotFound  -> print("Not found"),
        Error(e)  -> print("Error: ", e),
        _         -> print("Unknown"),
    }
}
```

---

## 🔹 **Example 4: Low-Level Memory + Inline ASM**

```afml
import forge.mem as mem;

fun apex() {
    var buf = mem.alloc(64);

    assembly {
        mov rax, 42
        mov [buf], rax
    }

    print(mem.read_i64(buf));

    mem.free(buf);
}
```

---

## 🔹 **Example 5: Math + Physics**

```afml
import forge.math as math;
import forge.physics as phys;
import forge.log as log;

fun apex() {
    let h = phys.height(10.0);         // height = 10 meters
    let t = phys.time_of_fall(h);

    log.info("Fall time = ", t, " s");
}
```

---

---

# 4. **FORMAL SYNTAX (EBNF) — REAL COMPILER READY**

Below is the clean and complete grammar you can directly use in a parser.

---

## 🔹 **4.1 LEXICAL**

```
letter      = "A"…"Z" | "a"…"z" ;
digit       = "0"…"9" ;
ident_start = letter | "_" ;
ident_char  = letter | digit | "_" ;

identifier  = ident_start , { ident_char } ;
```

**NO Unicode allowed**.

---

## 🔹 **4.2 TOKENS**

```
integer     = digit , { digit | "_" } ;
float       = digit , { digit } , "." , digit , { digit } ;
string      = '"' , { character } , '"' ;
char        = "'" , character , "'" ;
```

---

## 🔹 **4.3 FILE STRUCTURE**

```
file        = { import_stmt } , { top_level } ;
top_level   = function_def
            | struct_def
            | enum_def
            | trait_def
            | impl_def
            ;
```

---

## 🔹 **4.4 IMPORTS**

```
import_stmt = "import" , identifier ,
              { "::" , identifier } ,
              [ "as" , identifier ] , ";" ;
```

---

## 🔹 **4.5 TYPES**

```
type =
      identifier
    | identifier "<" type_list ">"
    | "[" type ";" integer "]"
    | "slice" "<" type ">"
    | "tuple" "(" type_list ")"
    ;

type_list = type , { "," , type } ;
```

---

## 🔹 **4.6 VARIABLES**

```
var_decl = ("let" | "var") ,
           identifier ,
           [ "::" , type ] ,
           "=" , expr , ";" ;
```

---

## 🔹 **4.7 FUNCTIONS**

```
function_def =
      [ "async" ] ,
      "fun" , identifier ,
      "(" , [ param_list ] , ")" ,
      [ "->" , [ "async" ] , type ] ,
      block ;

param_list = param , { "," , param } ;
param      = identifier , "::" , type ;
```

---

## 🔹 **4.8 EXPRESSIONS**

```
expr =
      literal
    | identifier
    | expr , binary_op , expr
    | "-" , expr
    | identifier "(" [ arg_list ] ")"
    | "(" expr ")"
    | block_expr
    ;
```

---

## 🔹 **4.9 BLOCK**

```
block = "{" , { stmt } , "}" ;
stmt  = var_decl
      | expr , ";"
      | return_stmt
      | if_stmt
      | while_stmt
      | for_stmt
      | switch_stmt
      ;

return_stmt = "return" , [ expr ] , ";" ;
```

---

## 🔹 **4.10 STRUCTS**

```
struct_def =
    "struct" , identifier , "{",
        { identifier , "::" , type , "," } ,
    "}" ;
```

---

## 🔹 **4.11 ENUMS**

```
enum_def =
    "enum" , identifier , "{",
        variant , { "," , variant } ,
    "}" ;

variant = identifier [ "(" , type_list , ")" ] ;
```

---

## 🔹 **4.12 CONTROL FLOW**

### IF

```
if_stmt = "if" , expr , block ,
          { "else if" , expr , block } ,
          [ "else" , block ] ;
```

### WHILE

```
while_stmt = "while" , expr , block ;
```

### FOR

```
for_stmt = "for" , identifier , "in" , expr , block ;
```

### SWITCH

```
switch_stmt =
    "switch" , expr , "{",
        case , { "," , case },
    "}" ;

case = pattern , "->" , expr ;
```

---

---

# 5. **LEXICAL RULES (EXTRA)**

### Comments

```
"//" … end_of_line
"/*" … "*/"
```

### Allowed Characters

* Letters: ASCII only
* Numbers
* `_`

Forbidden:

* ∑ ∇ α β γ …
* Emoji
* Symbols like ₿ ¥ ※

---

---

# 6. **DATA TYPES (EXTENDED)**

### Primitive

```
i8 i16 i32 i64 i128
u8 u16 u32 u64 u128
f32 f64
bool
char
str
```

### Composite

```
[T;N]        fixed array
slice<T>
vec<T>
tuple(T1,T2,...)
option<T>
result<T,E>
```

---

---

# 7. **MEMORY MODEL**

### Ownership (Rust-like)

* One owner
* Moves on assignment
* Borrowing via `&T` and `&mut T`

---

### Smart Pointers

```
box<T>
rc<T>
arc<T>
weak<T>
```

---

### Raw Pointers

```
ptr<T>
ptr_mut<T>
```

---

### Memory Ops

```
mem.alloc(size)
mem.free(ptr)
mem.copy(dst, src, len)
mem.set(ptr, value, len)
mem.zero(ptr, len)
```

---

### Inline Assembly

```afml
assembly {
    mov rax, 10
    add rax, 20
}
```

---

---

# 8. **FUNCTIONS & ASYNC**

### Sync

```afml
fun add(a:: i32, b:: i32) -> i32 { ... }
```

### Async

```afml
async fun load() -> async str {
    ...
}
```

### Await

```afml
let result = await some_async_call();
```

---

---

# 9. **CONTROL FLOW**

### Switch

```afml
switch x {
    0 -> print("zero"),
    1 -> print("one"),
    _ -> print("other"),
}
```

### Try/Catch

```afml
try {
    risky();
} catch(e) {
    log.error(e);
}
```

---

---

# 10. **MODULES**

### Import single

```afml
import forge.math;
```

### Import element

```afml
import forge.crypto::sha256;
```

### With alias

```afml
import forge.fs as fs;
```

---

---

# 11. **ERROR HANDLING**

### Result

```afml
fun read(p:: str) -> result<str,error> { ... }
```

### Error propagation

```afml
let x = read("cfg.txt")?;
```

---

---

# 12. **STANDARD LIBRARY (EXTENDED)**

## **12.1 Strings**

```
len()
trim()
to_upper()
to_lower()
split()
replace()
reverse()
to_int()
to_float()
repeat()
find()
```

Regex:

```
re.match()
re.findall()
re.replace()
```

---

## **12.2 Math**

```
sin cos tan
exp ln log
sqrt pow
clamp lerp
gamma beta
sigmoid tanh
```

Linear Algebra, Calculus, Statistics INCLUDED.

---

## **12.3 Physics**

```
kinetic_energy
potential_energy
lorentz_factor
doppler_shift
```

Units included:

```
meter second kilogram watt joule volt ampere newton
```

---

## **12.4 Collections**

```
vec push pop sort map filter reduce
set insert remove contains
map put get keys values
```

---

## **12.5 Async**

```
async.all
async.any
async.race
async.timeout
async.retry
async.interval
```

---

## **12.6 Net**

```
http.get post put delete
tcp.connect
udp.sendto
ws.connect
dns.lookup
```

---

## **12.7 Crypto**

```
sha256
sha512
aes.encrypt
aes.decrypt
rsa.generate
ed25519.sign
```

---

## **12.8 OS**

```
os.sleep
os.time.now
os.env.get
```

---

## **12.9 FS**

```
read_file
write_file
append
exists
copy
move
```

---

## **12.10 GUI (Flutter-like)**

```afml
f.run_app(MyApp {});
```

Widget Tree:

```
Text
Button
Column
Row
Container
AppBar
Scaffold
Center
```

---

---

# 13. **COMPILER: HOW MODULES ARE ADDED**

To add new stdlib:

```
std/
   forge/
      math.afml
      fs.afml
      net/
      crypto/
      async/
```

Your compiler must:

1. Load `std/forge/**/*.afml`
2. Register them before `user code`
3. Allow `import forge.xxx`

---

---

# 14. **REAL PROJECT STRUCTURE**

```
my_project/
  afpm.toml
  src/
    main.afml
    utils.afml
    math/
      mod.afml
      calculus.afml
  std/   <-- only for compiler
```

---

---

# 15. **FUTURE EXTENSIONS**

* Compile-time evaluation: `meta {}`
* Contracts:

```
requires x > 0
ensures result >= 0
```

* Quantum module
* Bytecode VM
* AFNS → LLVM IR backend

---

---

# 🔥 **ApexForge NightScript – FULL STANDARD LIBRARY SPEC (ALL MODULES COMPLETED)**

*(Bu hissə bütün kitabxanaların tam API sənədidir — AFNS-in CORE STD LIB)*

---

# ✅ **1. forge.android — FULL ANDROID LIBRARY**

AFNS-in Android modulu **Java/NDK + JNI** əsasında işləyir.
AFNS Android API-sinə *Flutter + Kotlin + Java* qarışığı kimi baxa bilərsən.

---

## 1.1. Android Lifecycle

```afml
import forge.android.app as app;

trait Activity {
    fun on_create(ctx:: Context);
    fun on_start(ctx:: Context);
    fun on_resume(ctx:: Context);
    fun on_pause(ctx:: Context);
    fun on_stop(ctx:: Context);
    fun on_destroy(ctx:: Context);
}
```

---

## 1.2. App Entry (Android)

```afml
app.run(MyApp {});
```

```afml
struct MyApp {}

impl app::Activity for MyApp {
    fun on_create(ctx:: app::Context) {
        ctx.show_toast("Hello Android from AFNS!");
    }
}
```

---

## 1.3. UI API (native Android widgets)

```afml
ctx.set_view(
    ui::Column([
        ui::Text("Hello"),
        ui::Button("Click", fun(){ print("Pressed"); })
    ])
);
```

### Widgets:

```
Text
Button
Image
TextField
Switch
Slider
Row
Column
Stack
ScrollView
Card
AppBar
Scaffold
ListView
```

---

## 1.4. Android Permissions

```afml
app.permissions.request("android.permission.CAMERA")
```

Check:

```afml
if app.permissions.is_granted("CAMERA") { ... }
```

---

## 1.5. Android Intents

```afml
app.intent.send("android.intent.action.VIEW", url="https://google.com");
```

---

## 1.6. Android Services

```afml
app.service.start(MyService {});
```

---

## 1.7. File & Storage (Android)

```afml
app.storage.get_internal_path()
app.storage.get_external_path()
```

---

## 1.8. Java FFI for Android classes

```afml
@ffi("java:android.os.Build")
extern "Java" fun MODEL() -> str;

fun apex() {
    print("Device model:", MODEL());
}
```

---

# ✅ **2. forge.syscall — SYSTEM CALL INTERFACE**

Direct syscalls (Linux, Android):

```afml
syscall.getpid() -> i32
syscall.getuid() -> i32
syscall.write(fd:: i32, data:: str)
syscall.read(fd:: i32, size:: usize) -> bytes
syscall.open(path:: str, flag:: i32) -> i32
syscall.close(fd:: i32)
syscall.fork() -> i32
syscall.exec(path:: str, args:: vec<str>)
```

Raw syscall ID interface:

```afml
syscall.raw(id:: i32, arg1, arg2, arg3) -> i64
```

---

# ✅ **3. forge.io — INPUT/OUTPUT LIBRARY (FULL)**

### File I/O

```
io.read(path) -> result<str,error>
io.read_bytes(path)
io.write(path, str)
io.write_bytes(path, bytes)
io.append(path, str)
io.stream.read()
io.stream.write()
io.file.open()
```

### Network I/O

```
io.net.stream
io.net.buffered_stream
```

### Memory I/O

```
io.mem.read(ptr, size)
io.mem.write(ptr, data)
```

### Device I/O (embedded)

```
io.device.open(id)
io.device.read()
io.device.write()
```

---

# ✅ **4. forge.db — DATABASE SUPPORT**

## SQL

```afml
db.sql.connect("sqlite://test.db")
```

Common API:

```
conn.execute("CREATE TABLE ...")
conn.query("SELECT * FROM users")
conn.prepare("INSERT INTO users VALUES (?,?)")
stmt.bind(1, "Natiq")
stmt.bind(2, 20)
stmt.run()
```

Supported drivers:

```
sqlite
postgres
mysql
mariadb
```

---

## NoSQL

### Redis

```
db.redis.connect(...)
db.redis.set(key, val)
db.redis.get(key)
```

### MongoDB

```
db.mongo.connect(...)
db.mongo.insert(coll, doc)
db.mongo.find(coll, filter)
```

---

# ✅ **5. forge.ffi — FOREIGN FUNCTION INTERFACE**

### C FFI

```afml
@ffi("libm.so")
extern "C" fun sin(x:: f64) -> f64;
```

### Rust FFI

```afml
@ffi("librust.so")
extern "Rust" fun rust_func(x:: i32);
```

### Java FFI

```afml
@ffi("java:java.lang.System")
extern "Java" fun currentTimeMillis() -> i64;
```

### Unsafe Raw Pointers

```afml
extern "C" fun memcpy(dst:: ptr<u8>, src:: ptr<u8>, len:: usize);
```

---

# ✅ **6. forge.types — BUILTIN SPECIAL TYPES**

```
uuid
email
IpAddr
MacAddr
url
date
datetime
timezone
path
color
```

Example:

```afml
let id = uuid::v4();
let email = email::parse("user@example.com")?;
```

---

# ✅ **7. forge.error — ERROR SYSTEM**

### Create custom error:

```afml
error.new("FileNotFound")
```

### Throw error:

```afml
error.throw("BadInput")
```

### Handle:

```afml
try { risky() } catch(e) { log.error(e) }
```

### Convert to result:

```afml
result<T,E>
option<T>
```

---

# ✅ **8. forge.serde — SERIALIZATION FRAMEWORK**

### JSON

```afml
serde.json.encode(obj)
serde.json.decode<Struct>(str)
```

### YAML

```afml
serde.yaml.encode(...)
```

### XML

```afml
serde.xml.encode(...)
```

### Binary (MessagePack-like)

```
serde.bin.encode
serde.bin.decode
```

---

# ✅ **9. forge.net — NETWORK LIBRARY (FULL)**

## HTTP

```
http.get(url)
http.post(url, json=data)
http.put(...)
http.delete(...)
```

### Response API:

```
resp.status()
resp.text()
resp.json<T>()
resp.bytes()
```

### Client Object

```
client = http.client(timeout=5)
client.get(...)
```

---

## WebSocket

```
ws.connect(url)
ws.send("hi")
ws.recv()
ws.close()
```

---

## TCP

```
tcp.listen(port)
tcp.accept()
tcp.connect(addr)
```

---

## UDP

```
udp.bind(port)
udp.sendto(data, addr)
udp.recvfrom()
```

---

## DNS

```
dns.lookup("google.com")
```

---

# ✅ **10. forge.os — OS INFORMATION**

### System

```
os.cpu_count()
os.memory_info()
os.disk_info()
os.process_id()
os.thread_id()
os.sleep(ms)
```

### Time

```
os.time.now()
os.time.unix()
os.time.format(datetime)
```

---

# 🔹 forge.os.env – ENVIRONMENT API

```
env.get("PATH")
env.set("EDITOR", "vim")
env.vars()
```

---

# ✅ **11. forge.fs — FILESYSTEM**

Synchronous, portable filesystem helpers (all fallible ops return `result<_, str>`).

```
fs.read_to_string(path)        -> result<str, str>
fs.read_bytes(path)            -> result<vec<u8>, str>
fs.write_string(path, data)    -> result<(), str>
fs.write_bytes(path, vec<u8>)  -> result<(), str>
fs.append_string(path, data)   -> result<(), str>
fs.append_bytes(path, vec<u8>) -> result<(), str>

fs.create_dir(path)            -> result<(), str>
fs.create_dir_all(path)        -> result<(), str>
fs.remove_dir(path)            -> result<(), str>
fs.remove_dir_all(path)        -> result<(), str>

fs.exists(path)                -> bool
fs.is_file(path)               -> bool
fs.is_dir(path)                -> bool
fs.metadata(path)              -> result<fs::FsMetadata, str>
fs.read_dir(path)              -> result<vec<fs::DirEntry>, str>

fs.join(base, child)           -> str
fs.dirname(path)               -> str
fs.basename(path)              -> str
fs.extension(path)             -> option<str>
fs.canonicalize(path)          -> result<str, str>
fs.is_absolute(path)           -> bool
fs.strip_prefix(base, path)    -> result<str, str>

fs.copy_file(src, dst)         -> result<(), str>
fs.copy(src, dst)              -> result<(), str>  // alias
fs.copy_dir_recursive(src,dst) -> result<(), str>
fs.move(src, dst)              -> result<(), str>
fs.rename(src, dst)            -> result<(), str>  // alias
fs.remove_file(path)           -> result<(), str>
fs.ensure_dir(path)            -> result<(), str>
fs.read_lines(path)            -> result<vec<str>, str>
fs.write_lines(path, vec<str>) -> result<(), str>
fs.read_link(path)             -> result<str, str>
fs.is_symlink(path)            -> bool
fs.hard_link(src, dst)         -> result<(), str>
fs.symlink_file(src, dst)      -> result<(), str>
fs.symlink_dir(src, dst)       -> result<(), str>
fs.chmod(path, mode)           -> result<(), str> (unix modes; best-effort on Windows)
fs.symlink_metadata(path)      -> result<fs::FsMetadata, str>
fs.components(path)            -> result<vec<str>, str>
fs.parent(path)                -> option<str>
fs.file_stem(path)             -> option<str>
fs.touch(path)                 -> result<(), str>
fs.copy_permissions(src,dst)   -> result<(), str>
fs.current_dir()               -> result<str, str>
fs.temp_dir()                  -> str
fs.temp_file()                 -> result<str, str>
```

### forge.net (sync std::net)
`tcp_connect`, `tcp_listen`, `tcp_accept`, `tcp_send`, `tcp_recv`, `tcp_shutdown`, `tcp_set_nodelay`, `tcp_set_read_timeout`, `tcp_set_write_timeout`, `tcp_peer_addr`, `tcp_local_addr`, `udp_bind`, `udp_connect`, `udp_send`, `udp_send_to`, `udp_recv`, `udp_recv_from`, `udp_set_broadcast`, `udp_set_read_timeout`, `udp_set_write_timeout`, `udp_peer_addr`, `udp_local_addr`, `close_socket`, `close_listener`. Types: `net::Socket`, `net::Listener`, `net::UdpSocket`, `net::UdpPacket { data, from }`.

### forge.db (sqlite + postgres + redis)
- `db.open(kind:: str, target:: str) -> result<db::Connection, str>`
  - `kind = "sqlite"` → `target = "path/to.db"` or `sqlite:path`
  - `kind = "postgres"` → `target = "postgres://user:pass@host:port/db"`
  - `kind = "redis"` → `target = "redis://host:port/db"`
- SQL operations (`sqlite`, `postgres`):
  - `db.exec(conn, sql) -> result<db::ExecResult, str>` (`rows_affected`)
  - `db.query(conn, sql) -> result<vec<map<str, Value>>, str>`
  - `db.begin(conn)`, `db.commit(conn)`, `db.rollback(conn)` → `result<(), str>`
- Key-value operations (`redis`):
  - `db.set(conn, key, value) -> result<(), str>`
  - `db.get(conn, key) -> result<option<str>, str>`
  - `db.del(conn, key) -> result<i64, str>`
- `db.close(conn) -> result<(), str>`

Structured types exposed to AFNS:

- `fs::FsMetadata { is_file, is_dir, size, readonly, created_at?, modified_at?, accessed_at? }`
- `fs::DirEntry { path, file_name, is_file, is_dir }`
- `db::Connection { id }`
- `db::ExecResult { rows_affected }`

---

## ✅ **Registry & Packages (Phase 1)**

- **Server**: `cargo run --manifest-path nightscript-server/Cargo.toml` starts the local registry on `127.0.0.1:5665`.
- **Auth flow**:
  - `curl -X POST /api/v1/register` to create a user.
  - `apexrc login --registry http://127.0.0.1:5665` (stdin prompts username/password).
  - `apexrc whoami` shows the authenticated user + registry URL.
- **Publishing AFML libraries**:
  - `apexrc init --crates` scaffolds `Apex.toml` with `[registry]`.
  - `apexrc publish` packages `Apex.toml`, `README.md`, `src/**` into `.apkg`, computes SHA-256, and uploads via `/api/v1/packages/publish`.
- **Consuming packages**:
  - `apexrc add hello-afml` records a semver constraint.
  - `apexrc install` resolves highest compatible versions, verifies checksums, caches under `~/.apex/pkgs/<name>/<version>`, and vendors into `./target/vendor/afml/<name>@<version>/`.
  - `apexrc update` refreshes dependencies to the latest allowed versions; `apexrc install --locked` uses `Apex.lock` verbatim.
  - Builds/runs automatically call `apexrc install --locked` to ensure the vendor tree matches the lockfile.
- **Diagnostics**: `scripts/phase1_e2e.sh` runs an end-to-end test (server → publish → add/install → build/run) and asserts that the application prints the library output.

See `examples/registry_demo/README.md` for a hands-on walkthrough.

### Registry UI & HTML pages

- `cargo run --manifest-path nightscript-server/Cargo.toml` serves both the JSON API and a Tailwind-powered HTML UI at `http://127.0.0.1:5665`.
- Landing page (`/`) highlights quick links plus a search box; `/packages` supports substring search + pagination directly through query params (`?q=net&page=2&per_page=50&sort=updated|name`).
- `/package/<name>` renders README.md straight from the uploaded tarball and shows install commands, version history, checksums, and download links.
- `/owner/<handle>` shows a profile-like list of packages owned by a user; `/login` exposes a dev-only token helper form.
- Every HTML page sets `Last-Modified`/`ETag`, and JSON equivalents remain at `/api/v1/*` for automation.
- Screenshot (current Tailwind layout):

  ![ApexForge Registry UI](assets/branding/apexforge_logo.png)

### Manifest metadata & multi-target packages

`Apex.toml` now captures richer package metadata plus optional targets so libraries written in AFML, Rust, or Java can advertise their build entry points. All fields remain optional for backward compatibility—existing manifests without `[targets.*]` continue to publish, while new manifests can opt in:

```toml
[package]
name = "hello-afml"
version = "0.2.0"
license = "MIT"
description = "Example package"
keywords = ["hello", "demo"]
homepage = "https://apexforge.dev/hello-afml"
repository = "https://github.com/apexforge/hello-afml"
readme = "README.md"
min_runtime = ">=1.0.0"

[targets.afml]
entry = "src/lib.afml"

[targets.rust]
crate = "hello_afml"
lib_path = "rust/Cargo.toml"
build = "cargo build -p hello_afml --release"

[targets.java]
gradle_path = "java/build.gradle"
group = "dev.apexforge"
artifact = "hello-afml"
version = "0.2.0"
build = "./gradlew jar"
```

`apexrc publish` automatically serializes the manifest to JSON (`manifest_json`) and uploads it alongside the `.apkg`. The registry persists the full package metadata plus the per-version target matrix so both the HTML UI and `/api/v1/package/*` JSON endpoints can display badges like “Targets: AFML · Rust · Java.” Older clients that only send the TOML payload are still accepted—the server treats missing sections as defaults.

### Tooling & VS Code integration

- `apexrc check` is now wired into the official VS Code extension (located under `apexforge-nightscript-vscode/`). The editor automatically runs lightweight parse checks on file open/save, surfaces diagnostics through the Problems panel, and mirrors the CLI output inside the “ApexForge apexrc” output channel. Trigger the pass manually via **Command Palette → ApexForge: Run apexrc check**.
- Configure the CLI path/arguments with `apexforge.apexrcPath` and `apexforge.apexrcCheckArgs` inside VS Code settings when working in sandboxes or with custom toolchains.
- The VSIX README documents every feature (syntax, snippets, completions, icons, diagnostics) and credits the ApexForge branding assets so teams know the extension is officially sanctioned.
- `apexrc` + VSIX + registry now form a Cargo-parity workflow: edit `.afml` files with linting, `apexrc check` validates AST/lexer errors, `apexrc build|run` emit ELF binaries, and `apexrc publish/add/install` talks to the local registry described below.
- `apexrc install` reads `.afml/exports.json` for every dependency (each entry now describes `targets` plus an optional `java_class`/signature hint), copies it into `target/vendor/afml/<name>@<version>`, and updates `target/vendor/.index.json`. Native dependencies drop compiled libraries into `.afml/lib/<triplet>/`, while Java dependencies drop shaded JARs under `.afml/java/`. The runtime module loader binds Rust exports via `libloading` and invokes Java exports through JNI using the declared `java_class` metadata. `apexrc doctor` inspects every vendor entry, prints the native library status, the resolved `.afml/java` JAR path, and the host `java -version` output so you can fix missing artifacts before execution.

### Forge standard library snapshot

- **forge.fs** — complete filesystem + path API: `read_to_string`, `read_bytes`, `write_string`, `write_bytes`, `append_*`, directory creation/removal (single + recursive), safe delete helpers, `copy_file`, `move`, metadata queries (`FsMetadata` with size/readonly/timestamps), and high-level utilities (`ensure_dir`, `read_lines`, `write_lines`, `copy_dir_recursive`, `join`, `dirname`, `basename`, `extension`, `canonicalize`). All operations use `result<…, str>` and integrate with Phase‑5 generics.
- **forge.net** — synchronous TCP/UDP surface with the full socket toolkit: `tcp_connect`, `tcp_listen`, `tcp_accept`, send/recv helpers, graceful shutdown, per-socket flags (`set_nodelay`, read/write timeouts), address introspection, and UDP wrappers (`udp_bind`, `udp_send_to`, `udp_recv_from`, broadcast toggles). The native backend lowers these calls to Linux syscalls.
- **forge.db** — multi-backend database layer: SQLite (`rusqlite`), PostgreSQL (`sqlx`), and Redis (`redis-rs`). AFNS code can `db.open`, `db.exec`, `db.query`, `db.begin/commit/rollback`, and issue key–value operations (`db.get/set/del`). Example projects (`examples/db_*`) showcase real connections.
- **forge.async** — Tokio-powered runtime glue plus convenience intrinsics (`async.parallel`, `async.race`, `async.all`, `async.interval`, `async.retry`, etc.) that all ship with snippets/completions.
- **forge.log / Flutter UI / generics** — logging macros, Flutter-style widget DSL (`ctx.*`), and the generic-heavy samples (`examples/generics_*`, `examples/custom_generic_type`) continue to inform snippets/completions so every forge module behaves consistently across compiler + VSIX tooling.

#### forge.fs quick reference

- File IO: `read_to_string`, `read_bytes`, `write_string`, `write_bytes`, `append_string`, `append_bytes`, `touch`, `remove_file`.
- Directories: `create_dir`, `create_dir_all`, `remove_dir`, `remove_dir_all`, `ensure_dir`.
- Utilities: `copy_file`, `move`, `copy_dir_recursive`, `read_dir`, `read_lines`, `write_lines`.
- Metadata & paths: `metadata`, `symlink_metadata`, `exists`, `is_file`, `is_dir`, `join`, `dirname`, `basename`, `extension`, `parent`, `canonicalize`.
- Structures: `fs::FsMetadata` (size, timestamps, readonly flags) and `fs::DirEntry` (path, file_name, type markers).

#### forge.net quick reference

- TCP: `tcp_connect`, `tcp_listen`, `tcp_accept`, `tcp_send`, `tcp_recv`, `tcp_shutdown`, `tcp_set_nodelay`, `tcp_set_read_timeout`, `tcp_set_write_timeout`, `tcp_peer_addr`, `tcp_local_addr`.
- UDP: `udp_bind`, `udp_connect`, `udp_send`, `udp_send_to`, `udp_recv`, `udp_recv_from`, `udp_set_broadcast`, `udp_set_read_timeout`, `udp_set_write_timeout`, `udp_peer_addr`, `udp_local_addr`.
- Lifetimes: `close_socket`, `close_listener` ensure descriptors are released deterministically.

#### forge.db quick reference

- Connection mgmt: `db.open(kind, target)`, `db.close(conn)` for SQLite/Postgres/Redis URIs.
- SQL helpers: `db.exec`, `db.query`, `db.begin`, `db.commit`, `db.rollback`.
- Key-value helpers (Redis): `db.get`, `db.set`, `db.del`.
- Result types: `db::ExecResult { rows_affected }`, query results as `vec<map<str, Value>>`, and redis-style `option<Value>`.

#### forge.async + Flutter DSL

- Async primitives: `async.parallel`, `async.race`, `async.all`, `async.any`, `async.interval`, `async.retry`, `async.sleep`, `async.timeout`, `async.spawn`, etc.
- UI snippets: `ctx.text`, `ctx.button`, `ctx.column`, `ctx.row`, `ctx.scaffold`, `ctx.image`, `ctx.list`, `ctx.widget`.
- Logging: `forge.log.info`, `.warn`, `.error` macros appear in completions/snippets so instrumentation is one keystroke away.

---

## ✅ **Dependency Resolution**
 
- Supports the full semver grammar: caret (`^`), tilde (`~`), exact (`=`), inequality ranges (`>=`, `<=`, `<`, `>`), and wildcards (`*`, `1.*`). Conflicts surface with a list of the constraints that disagreed so it is easy to fix manifests.
- `Apex.lock` stores the entire resolved graph (direct + transitive) deterministically. Locked builds (`apexrc build/run/perf`) refuse to touch the network if the lockfile is missing or stale—run `apexrc install`/`apexrc update` to refresh.
- CLI parity with Cargo:
  - `apexrc add foo@^1.0` / `apexrc remove foo` edit `Apex.toml`, resolve, and vendor under `target/vendor/afml/<name>@<version>/`.
  - `apexrc update` refreshes every dependency, while `apexrc update foo bar` only unlocks the listed roots (everything else stays pinned to the lock).
  - `apexrc outdated` prints available upgrades; `apexrc tree` renders the lockfile dependency tree; `apexrc why foo` explains why `foo` was selected (constraints + dependency paths).
  - `apexrc install --locked` installs strictly from `Apex.lock`; omitting `--locked` re-runs the solver and rewrites the lockfile.
- Vendor pruning happens automatically—unused packages are removed from `target/vendor/afml/` after each resolve cycle, with download progress bars provided by `indicatif` (use `--quiet` to suppress them).
 
Example workflow:
 
```bash
apexrc add forge.fs@^1.2
apexrc install
apexrc update forge.fs
apexrc outdated
apexrc tree
apexrc why forge.fs
```

Sample output:

```bash
$ apexrc tree
├── forge.fs@1.2.1 (req ^1.2)
└── forge.log@0.5.0 (req =0.5.0)

$ apexrc why forge.fs
forge.fs @ 1.2.1 (07fdd…)
Constraints:
  - registry-app (manifest) requires ^1.2
  - forge.shell@0.3.0 requires >=1.2, <2
Dependency paths:
  - registry-app -> forge.shell -> forge.fs
```
 
---

# ✅ **12. forge.collections — FULL COLLECTIONS**

## Vector

```
vec.push()
vec.pop()
vec.insert()
vec.remove()
vec.extend()
vec.map()
vec.filter()
vec.reduce()
vec.sort()
vec.reverse()
```

## Set

```
set.insert()
set.contains()
set.remove()
set.union()
set.intersection()
```

## Map/Dict

```
map.put()
map.get()
map.remove()
map.keys()
map.values()
map.items()
```

## RingBuffer

```
ring.push()
ring.pop()
```

## Buffer / ByteBuffer

```
buf.read()
buf.write()
buf.seek()
```

---

## Credits

Created by **Natiq Mammadov — ApexForge**  
GitHub: https://github.com/Natiqmammad

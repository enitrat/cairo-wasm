# Cairo WASM

Compile and run Cairo programs in the browser. Two crates expose the Cairo
compiler and runner as WebAssembly modules with JSON APIs, so a web application
can compile Cairo source to Sierra and execute it without a backend server.

| Crate                      | Purpose                                           |
| -------------------------- | ------------------------------------------------- |
| `cairo-lang-compiler-wasm` | Compile Cairo source to Sierra                    |
| `cairo-lang-runner-wasm`   | Compile and run Cairo, or run pre-compiled Sierra |

Both crates embed the full Cairo corelib at build time. A browser app ships
self-contained — no filesystem access, no corelib resolution, no external
dependencies at runtime.

---

## Part 1 — Integration Guide

### Building the WASM Packages

From the repository root:

```bash
wasm-pack build crates/cairo-lang-compiler-wasm --target web --release
wasm-pack build crates/cairo-lang-runner-wasm  --target web --release
```

This produces JS/WASM artifacts under each crate's `pkg/` directory:

```
crates/cairo-lang-compiler-wasm/pkg/
crates/cairo-lang-runner-wasm/pkg/
```

Import the generated ES modules directly into your application.

### Exported Functions

**Compiler crate** (`cairo-lang-compiler-wasm`):

| Function                               | Description                      |
| -------------------------------------- | -------------------------------- |
| `compile(requestJson: string): string` | Compile Cairo source to Sierra   |
| `embedded_corelib_manifest(): string`  | List embedded corelib file paths |

**Runner crate** (`cairo-lang-runner-wasm`):

| Function                                       | Description                           |
| ---------------------------------------------- | ------------------------------------- |
| `compile_and_run(requestJson: string): string` | Compile Cairo source and execute it   |
| `run_sierra(requestJson: string): string`      | Execute a pre-compiled Sierra program |
| `embedded_corelib_manifest(): string`          | List embedded corelib file paths      |

Every function accepts a JSON string and returns a JSON string.

---

### Compile API

#### Request

```json
{
    "crate_name": "app",
    "files": {
        "lib.cairo": "fn main() -> felt252 { 7 }"
    },
    "replace_ids": true,
    "inlining_strategy": "default"
}
```

| Field               | Type           | Required | Default          | Description                                                     |
| ------------------- | -------------- | -------- | ---------------- | --------------------------------------------------------------- |
| `crate_name`        | string         | yes      | —                | Name for the virtual crate                                      |
| `files`             | object         | yes      | —                | Map of relative paths to Cairo source. Must include `lib.cairo` |
| `corelib_files`     | object \| null | no       | embedded corelib | Override the corelib with custom files                          |
| `replace_ids`       | bool           | no       | `false`          | Replace Sierra identifiers with human-readable names            |
| `inlining_strategy` | string         | no       | `"default"`      | `"default"` or `"avoid"`                                        |

#### Response

```json
{
    "success": true,
    "sierra": "type felt252 = felt252 ...",
    "diagnostics": "",
    "error": null
}
```

| Field         | Type           | Description                                                    |
| ------------- | -------------- | -------------------------------------------------------------- |
| `success`     | bool           | Whether compilation succeeded                                  |
| `sierra`      | string \| null | The Sierra program text on success, `null` on failure          |
| `diagnostics` | string         | Compiler warnings and notes (may be non-empty even on success) |
| `error`       | string \| null | Error description on failure                                   |

---

### Compile-and-Run API

#### Request

```json
{
    "crate_name": "app",
    "files": {
        "lib.cairo": "fn main() { println!(\"Hello World\"); }"
    },
    "available_gas": 1000000,
    "function": "::main"
}
```

| Field               | Type           | Required    | Default          | Description                                                                   |
| ------------------- | -------------- | ----------- | ---------------- | ----------------------------------------------------------------------------- |
| `crate_name`        | string         | yes         | —                | Name for the virtual crate                                                    |
| `files`             | object         | yes         | —                | Map of relative paths to Cairo source. Must include `lib.cairo`               |
| `corelib_files`     | object \| null | no          | embedded corelib | Override the corelib                                                          |
| `replace_ids`       | bool           | no          | `true`           | Replace Sierra identifiers (defaults to `true` here so `::main` lookup works) |
| `inlining_strategy` | string         | no          | `"default"`      | `"default"` or `"avoid"`                                                      |
| `available_gas`     | number \| null | conditional | —                | Gas budget. Required when the program uses gas accounting                     |
| `function`          | string         | no          | `"::main"`       | Fully-qualified function name to execute                                      |

#### Response

```json
{
    "success": true,
    "panicked": false,
    "values": ["7"],
    "stdout": "",
    "gas_counter": "999000",
    "diagnostics": "",
    "error": null
}
```

| Field         | Type           | Description                                                                      |
| ------------- | -------------- | -------------------------------------------------------------------------------- |
| `success`     | bool           | `true` when the program runs to completion without panicking                     |
| `panicked`    | bool           | Whether the Cairo program panicked                                               |
| `values`      | string[]       | Return values as stringified felts                                               |
| `stdout`      | string         | Captured output from `println!` calls                                            |
| `gas_counter` | string \| null | Remaining gas after execution                                                    |
| `diagnostics` | string         | Compiler diagnostics (empty when using `run_sierra`)                             |
| `error`       | string \| null | Infrastructure error — compilation failure, missing function, runner setup error |

---

### Run-Sierra API

The `run_sierra` endpoint accepts a pre-compiled Sierra program directly,
skipping the compilation step. Useful when the frontend caches compiled output
or receives Sierra from an external source.

#### Request

```json
{
    "sierra": "type felt252 = felt252 ...",
    "available_gas": 1000000,
    "function": "::main"
}
```

| Field           | Type           | Required    | Default    | Description                                   |
| --------------- | -------------- | ----------- | ---------- | --------------------------------------------- |
| `sierra`        | string         | yes         | —          | Sierra program text                           |
| `available_gas` | number \| null | conditional | —          | Gas budget (required if the program uses gas) |
| `function`      | string         | no          | `"::main"` | Function to execute                           |

The response schema is identical to the compile-and-run response above.

---

### Stdout Capture

`println!` in Cairo compiles down to `CoreHint::DebugPrint` hints executed
during VM interpretation. In a native terminal, these hints call `print!` and
the output goes to the process stdout — useless in a browser.

The WASM runner intercepts `DebugPrint` hints and appends their output to an
internal buffer while still calling the original `print!` path. That buffer
surfaces as the `stdout` field in the JSON response. This makes program output
deterministic and API-visible without relying on console scraping.

A program that does not call `println!` returns `stdout: ""`.

---

### Browser Example

```js
import initCompiler, { compile } from "./pkg-compiler/cairo_lang_compiler_wasm.js";
import initRunner, { compile_and_run } from "./pkg-runner/cairo_lang_runner_wasm.js";

// Initialize WASM modules
await initCompiler();
await initRunner();

// Compile to Sierra
const compileResult = JSON.parse(
    compile(
        JSON.stringify({
            crate_name: "app",
            files: { "lib.cairo": "fn main() -> felt252 { 42 }" },
            replace_ids: true,
        })
    )
);

if (compileResult.success) {
    console.log("Sierra:", compileResult.sierra);
}

// Compile and run
const runResult = JSON.parse(
    compile_and_run(
        JSON.stringify({
            crate_name: "app",
            files: { "lib.cairo": 'fn main() { println!("Hello from Cairo"); }' },
            available_gas: 1_000_000,
        })
    )
);

if (runResult.success) {
    console.log("stdout:", runResult.stdout); // "Hello from Cairo\n"
    console.log("values:", runResult.values);
}
```

### The `#[executable]` Attribute

Programs annotated with `#[executable]` work through both the compile and
compile-and-run paths. The WASM compiler loads Cairo's `executable_plugin_suite`
so entry-point detection and code generation behave identically to the native
CLI. No special configuration is needed — use `#[executable]` as you normally
would:

```json
{
    "crate_name": "app",
    "files": {
        "lib.cairo": "#[executable]\nfn main() { println!(\"Hello executable\"); }"
    },
    "available_gas": 1000000
}
```

---

## Part 2 — How Cairo Was Made WASM-Compatible

### The Starting Point

The Cairo compiler was never designed for the browser. It assumes a local
filesystem and its runner depends on OS-level randomness. Making it work as
WebAssembly meant identifying every assumption that breaks under
`wasm32-unknown-unknown` and finding the narrowest change that removes the
assumption without disrupting native builds.

The work divided into three areas: replacing the filesystem with an in-memory
project model, making the runner's VM execution work without OS facilities, and
making `println!` output observable through the API.

### The In-Memory Project Model

Cairo's compiler expected to read source files from disk. The filesystem layer
already supported virtual files and crates, but no API existed to configure an
entire project — main crate plus corelib — from in-memory maps.

The new `InMemoryProject` struct wraps this:

```rust
pub struct InMemoryProject {
    pub main_crate_name: String,
    pub main_crate_files: BTreeMap<String, String>,
    pub corelib_files: BTreeMap<String, String>,
    pub main_crate_settings: Option<CrateSettings>,
}
```

A caller provides a crate name, a map of relative paths to source strings, and
optionally custom corelib files. The `compile_in_memory_project` function sets
up virtual directories, registers the crate and corelib in the compiler
database, and runs the standard compilation pipeline. No paths touch the host
filesystem.

The corelib itself is embedded at build time. A `build.rs` script walks
`corelib/src/**/*.cairo` and generates a static array of `(path, content)`
pairs compiled into the WASM binary. When a request omits `corelib_files`, the
embedded copy is used automatically. This makes the WASM module entirely
self-contained — a single `.wasm` file carries the full Cairo standard library.

### Making the Runner Work in WASM

The runner needed one adaptation for `wasm32-unknown-unknown`: randomness.
`cairo-lang-runner` used `rand` with OS-backed entropy for the `RandomEcPoint`
hint. On WASM, the `getrandom` crate fails because there is no OS randomness
source. The fix is target-specific: on native builds, the runner still uses OS
randomness through the workspace `rand` configuration. On WASM, it uses
`SmallRng` seeded from a deterministic atomic counter. The output is not
cryptographically random, but the `RandomEcPoint` hint only needs structural
validity — it generates points for testing, not for key material.

### Stdout — Making println! Observable

In the native runner, `println!` output flows through `CoreHint::DebugPrint`,
which calls `print!` and writes to the process stdout. In the browser, there is
no process stdout. Even if `console.log` captured it, the output would be
interleaved with other messages and impossible to attribute to a specific
program run.

The solution adds a capture buffer to `CairoHintProcessor`. When processing
`DebugPrint` hints, the handler appends the formatted text to an internal
`String` buffer in addition to calling the original `print!`. After execution
completes, this buffer is carried through `RunResultStarknet.stdout` and
serialized into the JSON response. The existing native behavior is preserved —
`print!` still fires — but the output is now also available programmatically.

### What Remains

The current implementation is validated locally: unit tests pass on the host,
and both WASM crates compile for `wasm32-unknown-unknown`. The open work is
productization:

-   **Publishing strategy** — deciding whether to publish the WASM crates to npm
    via `wasm-pack` or distribute pre-built artifacts.
-   **JS ergonomics** — wrapping the raw JSON API in a typed TypeScript SDK.
-   **Browser demo** — a minimal playground that exercises compile and run.

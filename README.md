# Cairo WASM Integration Guide

## 1. Consumer-Facing Integration Instructions

This branch exposes two browser-oriented WASM crates:

- `crates/cairo-lang-compiler-wasm`: compile Cairo source to Sierra in-memory.
- `crates/cairo-lang-runner-wasm`: compile and run Cairo source (or run Sierra) in-memory.

### Build the WASM packages

From the repository root:

```bash
wasm-pack build crates/cairo-lang-compiler-wasm --target web --release
wasm-pack build crates/cairo-lang-runner-wasm --target web --release
```

This generates JS/WASM artifacts under:

- `crates/cairo-lang-compiler-wasm/pkg`
- `crates/cairo-lang-runner-wasm/pkg`

### API surface

`cairo-lang-compiler-wasm` exports:

- `compile(requestJson: string): string`
- `embedded_corelib_manifest(): string`

`cairo-lang-runner-wasm` exports:

- `compile_and_run(requestJson: string): string`
- `run_sierra(requestJson: string): string`
- `embedded_corelib_manifest(): string`

All functions accept JSON strings and return JSON strings.

### Request/response schema (compiler)

Compile request:

```json
{
  "crate_name": "app",
  "files": {
    "lib.cairo": "fn main() -> felt252 { 7 }"
  },
  "corelib_files": null,
  "replace_ids": true,
  "inlining_strategy": "default"
}
```

Compile response:

```json
{
  "success": true,
  "sierra": "...",
  "diagnostics": "",
  "error": null
}
```

### Request/response schema (runner)

Compile+run request:

```json
{
  "crate_name": "app",
  "files": {
    "lib.cairo": "fn main(){ println!(\"Hello World\"); }"
  },
  "available_gas": 1000000,
  "function": "::main"
}
```

Run response:

```json
{
  "success": true,
  "panicked": false,
  "values": [],
  "stdout": "Hello World\n",
  "gas_counter": "999000",
  "diagnostics": "",
  "error": null
}
```

Notes:

- `available_gas` is required for programs that use gas accounting.
- `function` defaults to `::main`.
- `corelib_files` is optional. If omitted, embedded corelib files are used.
- In runner compile+run, `replace_ids` defaults to `true` so `::main` lookup works reliably.

### How stdout becomes capturable

`println!` output in Cairo is produced through debug-print hints during execution. To make this observable for browser consumers, the runner now captures that text in-process:

- `CoreHint::DebugPrint` output is still printed, but is also appended to an internal stdout buffer.
- That buffer is carried through the run result as `RunResultStarknet.stdout`.
- `cairo-lang-runner-wasm` serializes this value into the JSON response field `stdout`.

This makes stdout deterministic and API-visible without scraping console output.

### Example browser usage

```js
import initCompiler, { compile } from "./cairo-lang-compiler-wasm/pkg/cairo_lang_compiler_wasm.js";
import initRunner, { compile_and_run } from "./cairo-lang-runner-wasm/pkg/cairo_lang_runner_wasm.js";

await initCompiler();
await initRunner();

const compileReq = {
  crate_name: "app",
  files: { "lib.cairo": "fn main() -> felt252 { 7 }" },
  replace_ids: true
};
const compileRes = JSON.parse(compile(JSON.stringify(compileReq)));

const runReq = {
  crate_name: "app",
  files: { "lib.cairo": "fn main(){ println!(\"Hello World\"); }" },
  available_gas: 1_000_000
};
const runRes = JSON.parse(compile_and_run(JSON.stringify(runReq)));
console.log({ compileRes, runRes });
```

## 2. How Cairo Was Made WASM-Compatible

The work started by separating what truly needed a full VM from what only needed a compiler pipeline. The compiler itself was already close to wasm-compatibility, but one transitive dependency path pulled in `cairo-vm` through code-size estimation logic. That coupling was broken for `wasm32` builds by target-gating the VM-dependent estimator and using a safe fallback estimator for the browser target.

From there, the larger architectural step was to stop treating the filesystem as mandatory input. A new in-memory project path was added so Cairo code and corelib files can be provided as maps of virtual paths to source strings. This made browser execution practical: no local files, no path assumptions, and no host filesystem APIs. With that in place, a dedicated `cairo-lang-compiler-wasm` crate wrapped compilation in a JSON API and embedded `corelib/src/**/*.cairo` at build time, so a browser app can compile immediately without shipping or resolving corelib separately.

Runner support was more involved. Two blockers surfaced immediately on `wasm32-unknown-unknown`: random-number plumbing and `cairo-vm` target configuration. The runner’s direct `rand` usage was adjusted with target-specific dependency settings, and the wasm path now uses deterministic `SmallRng` seeding for the relevant hint flow, avoiding unsupported OS randomness backends. In parallel, `cairo-vm` required a local patch: it forced `no_std` on wasm even when `std` should remain enabled. A vendored `cairo-vm` copy was patched so wasm builds can use the standard-library path, plus a couple of follow-up cleanup fixes needed for strict lint settings.

Once those blockers were removed, a second consumer crate, `cairo-lang-runner-wasm`, provided browser-facing compile+run and Sierra-run entry points with explicit JSON diagnostics and error reporting. Smoke tests validated both a simple value-returning Cairo program and a `println!(\"Hello World\")` program through the new API.

The result is a practical two-crate browser integration path: compile-only and compile+run, both in-memory, both corelib-embedded, both validated for `wasm32-unknown-unknown`. The remaining hardening work is mainly productization: publishing strategy, JS package ergonomics, browser demo UX, and deciding whether the temporary vendored `cairo-vm` patch should be upstreamed or replaced by an upstream release.

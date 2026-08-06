# fellowship-combat-log

TypeScript/JavaScript bindings for [`fellowship-combat-log`](../rust): a decoder for Fellowship's
Advanced Combat Log. Log text in, typed events out — usable from a browser or from Node.

This package wraps a small `wasm-bindgen` binding (`../rust` compiled to WebAssembly) with a
hand-written, fully-typed TypeScript API, so you get real types instead of `any` for the full
decoded event model.

## Install

```sh
npm install fellowship-combat-log
```

## Usage

```ts
import { parseLine, listCombatants, version } from "fellowship-combat-log";

const line =
  '2026-07-22T10:29:06.540+02:00|ABILITY_DAMAGE|Player-2000000001|"P2"|Npc-3049784064-42|"Desecrator"|2669|"Aura of Solace"|1|112|-5|-1|0|118|Magical|CriticalStrike|43441|43441|9385|0.0|0.0|0.0|[]|326373|326485|0|0.0|0.0|0.0|[]';

const event = await parseLine(1, line);
if (event.body.type === "DamageHeal") {
  console.log(event.body.result, event.body.applied); // "CriticalStrike" 112
}

// Malformed lines reject with an `Error`, so decode within a `try`/`catch` (or `.catch`):
try {
  await parseLine(2, "not a valid line");
} catch (err) {
  console.error(err); // Error: fellowship-combat-log: failed to parse line 2: ...
}

// Scan a whole log for its combatants and their latest gear snapshot. Unlike `parseLine`, this
// never throws — malformed lines are simply skipped.
const combatants = await listCombatants(fullLogText);
const you = combatants.find((c) => c.isRecordingPlayer);

console.log(await version()); // this decoder's version, e.g. "0.1.0"
```

Every export is async — see [Distribution target and caveats](#distribution-target-and-caveats)
for why. `parseLine`, `listCombatants`, and `version` each instantiate the wasm module on first
use and reuse the same instance after that; call `init()` yourself first if you'd rather pay that
cost up front (e.g. during your app's own startup) than on the first decode call.

The full discriminated-union event model — `Event`, `EventBody` and every event payload type
(`DamageHeal`, `Cast`, `Effect`, `CombatantInfo`, ...), plus `Guid`, `Combatant`, and every
supporting enum — is exported from this package with camelCase field names. See
[`ts/types.ts`](./ts/types.ts) for the full set; it mirrors the JSON [`src/bridge.rs`](./src/bridge.rs)
produces field for field.

A couple of representation notes worth knowing:

- An absent Rust `Option` is always JSON `null`, never an absent key — so optional fields are
  typed `T | null`, not `T | undefined`.
- Every numeric field, including the 64-bit ones (`applied`, `durationMs`, ...), comes across as a
  plain JS `number`. That's exact for any value real combat-log data produces, but it means a
  64-bit field whose value exceeds `Number.MAX_SAFE_INTEGER` (2^53 − 1) would fail to encode; in
  practice no field this crate decodes (timestamps, HP, damage/heal amounts, durations in
  milliseconds) gets remotely close to that.

## Build

```sh
npm install
npm run build       # wasm-pack build (release) + tsc
```

This runs two steps (also available individually as `npm run build:wasm` and
`npm run build:ts`):

1. `wasm-pack build --target web --out-dir pkg --out-name fellowship_combat_log --release` —
   compiles `src/` to WebAssembly and generates the raw JS/`.d.ts` glue into `pkg/` (gitignored,
   regenerated on every build).
2. `tsc -p tsconfig.json` — compiles the hand-written `ts/` entry point (which wraps `pkg/`'s raw,
   `any`-typed bindings with the types in `ts/types.ts`) to `dist/` (also gitignored).

Building `wasm-pack`'s target requires it to be installed (`cargo install wasm-pack`) and the
`wasm32-unknown-unknown` Rust target added (`rustup target add wasm32-unknown-unknown`).

`wasm-opt` (from the binaryen project) is deliberately disabled — see `[package.metadata.wasm-pack.profile.release]`
in `Cargo.toml`. Running it would fetch a prebuilt binaryen binary from a GitHub release at build
time, an extra network dependency and toolchain fetch this package skips in favor of relying on
rustc's own release-profile optimizations, which are plenty for a small, dependency-free crate
like this one. If you want the smaller binary `wasm-opt` produces, delete that section and rerun
the build (with network access).

## Test

```sh
npm test
```

Runs `npm run build` (via `pretest`) and then `node --test`, which discovers every `*.test.js`
file under `test/`. The tests decode the fixture logs in `../rust/tests/fixtures/` — the same
fixtures the Rust crate's own test suite uses — and assert on the decoded shape (including that
`listCombatants` finds the right combatants) and that a malformed line rejects.

## Distribution target and caveats

This package is built with `wasm-pack build --target web`, which produces an ES module that
exports an `init()` function to instantiate the wasm instead of doing it eagerly at import time.
That target was chosen because it is the one wasm-bindgen target that works, unmodified, in both
a browser (`fetch`-based instantiation) and — with the small adapter in `ts/index.ts` — Node
(reading the `.wasm` file's bytes directly and instantiating from them, bypassing `fetch`
entirely, since Node's built-in `fetch` does not support `file:` URLs). The alternative
single-target choices don't cover both: `--target nodejs` produces a CommonJS module that only
runs in Node, and `--target bundler` produces output that only works when it's actually run
through a bundler (it `import`s the `.wasm` file as if the build tooling will handle turning that
into an instantiated module, which plain Node and an unbundled browser `<script type="module">`
both cannot do on their own).

Caveats that follow from that choice:

- **This package is ESM-only** (`"type": "module"`). There is no CommonJS build; a CJS consumer
  needs a dynamic `import()`.
- **The wasm instantiation is asynchronous**, which is why every export here is `async` — a
  synchronous API isn't possible without embedding the wasm bytes as base64 in the JS (bloating
  the bundle) or requiring callers to pre-fetch bytes themselves. `init()` lets you control when,
  rather than whether, that `await` happens.
- **Bundling for the browser:** `ts/index.ts`'s Node-only code path (`import("node:fs/promises")`)
  is behind a runtime check and only ever executes under Node, but it's still a static import
  specifier that some browser-targeting bundler configurations may try to resolve. Modern
  bundlers (Vite, webpack 5, esbuild) generally handle this fine out of the box, either by
  tree-shaking the unreached branch or by treating unresolved `node:*` specifiers as external for
  a browser target; if yours doesn't, alias or externalize `node:fs/promises` and `node:url` for
  your browser build.
- Consuming this package **directly in an unbundled browser** `<script type="module">` works too:
  the `default` export in `pkg/fellowship_combat_log.js` resolves the `.wasm` file relative to its
  own `import.meta.url` and `fetch`es it, so no bundler step is required as long as `pkg/`'s files
  are served alongside your JS.

## License

MIT — see [`LICENSE`](./LICENSE).

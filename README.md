# fellowship-combat-log

Decodes [Fellowship](https://coffeestain.com/game/fellowship/)'s Advanced Combat Log into typed
events. Log text in, events out — nothing else.

The decoding logic lives once, in Rust, and every other language binds to it rather than
reimplementing it:

| Package | Language | What it is |
| --- | --- | --- |
| [`rust/`](rust) | Rust | The decoder itself: dependency-free, wasm32-compatible. [crates.io](https://crates.io/crates/fellowship-combat-log) |
| [`typescript/`](typescript) | TypeScript/JS | wasm-bindgen bindings over the same decoder, for browser and Node consumers. |
| [`python/`](python) | Python | PyO3 bindings over the same decoder, built as a native extension module. |

## Why one decoder, not three

`rust/` is the only place log-format knowledge lives. Its four modules — `timestamp → event →
parse → combatants` — decode a line into a typed event and nothing else, with no dependencies at
all, so it stays trivially auditable against the format and safe to embed anywhere, including
`wasm32-unknown-unknown`.

`typescript/` and `python/` are thin bindings: each depends on `rust/` by path, compiles it to
its own target (wasm for TypeScript, a native extension for Python), and exposes the same typed
event model idiomatically in its host language. Neither reimplements any parsing.

## Scope

Decoding only, in every language. Aggregation, validation, anonymization, encounter segmentation,
and resolving the game's numeric ids against a catalog all live in the consumer — see each
package's own README for its language-specific API.

## Licence

MIT.

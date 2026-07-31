# fellowship-combat-log

Decodes [Fellowship](https://www.fellowship-game.com/)'s Advanced Combat Log into typed Rust
events. Log text in, events out — nothing else.

```rust
use fellowship_combat_log::parse::parse_line;

for (index, line) in log_text.lines().enumerate() {
    match parse_line(index as u32 + 1, line) {
        Ok(event) => { /* event.instant, event.body */ }
        Err(error) => { /* a truncated or unknown line, never a panic */ }
    }
}
```

`combatants::list_combatants` scans a whole log for every character it mentions, each with their
latest equipped-gear snapshot.

## What it is, and what it deliberately is not

The crate is four modules forming a closed chain — `timestamp → event → parse → combatants` — with
no edge leaving the set. That closure is the point: it can be consumed on its own, without
inheriting anybody's application.

It has **no dependencies**, not even serde. Serializing a decoded run is a consumer's concern.

It is **wasm32-compatible**: no `std::fs`, no `SystemTime`, no native-only crates. All I/O belongs
to the caller, which is what lets the same decoder run in a browser and in a native tool.

`parse_line` is line-oriented, so tailing a log live needs no different API — only a caller that
owns the file handle.

Unknown event types are surfaced as `EventBody::Unknown`, never dropped, so a game update that
adds an event does not silently lose data. Malformed lines return an error rather than panicking,
because a live recording can be truncated mid-write and one bad line must not cost the whole read.

## Scope

Decoding only. Aggregation, validation grading, anonymization, encounter segmentation, and
resolving the game's numeric ids against catalogs all live in the consumer — those are
policy decisions an application makes, and folding them in here would narrow what the crate is
good for.

## Licence

MIT.

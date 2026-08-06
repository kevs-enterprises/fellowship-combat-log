# fellowship-combat-log

Decodes and encodes [Fellowship](https://coffeestain.com/game/fellowship/)'s Advanced Combat Log
against typed Rust events. Log text in, events out; events in, log text out — nothing else.

```rust
use fellowship_combat_log::parse::parse_line;

for (index, line) in log_text.lines().enumerate() {
    match parse_line(index as u32 + 1, line) {
        Ok(event) => { /* event.instant, event.body */ }
        Err(error) => { /* a truncated or unknown line, never a panic */ }
    }
}
```

```rust
use fellowship_combat_log::encode::encode_line;

match encode_line(&event) {
    Ok(line) => { /* valid v8 log-line text */ }
    Err(error) => { /* an event or field this build can't turn back into text, never a panic */ }
}
```

`combatants::list_combatants` scans a whole log for every character it mentions, each with their
latest equipped-gear snapshot.

## What it is, and what it deliberately is not

The crate is five modules forming a closed dependency graph — `timestamp → event → {parse, encode}
→ combatants` — with no edge leaving the set. That closure is the point: it can be consumed on its
own, without inheriting anybody's application.

It has **no dependencies**, not even serde — that holds for both directions.

It is **wasm32-compatible**: no `std::fs`, no `SystemTime`, no native-only crates. All I/O belongs
to the caller, which is what lets the same decoder/encoder run in a browser and in a native tool.

`parse_line` is line-oriented, so tailing a log live needs no different API — only a caller that
owns the file handle.

Unknown event types are surfaced as `EventBody::Unknown`, never dropped, so a game update that
adds an event does not silently lose data. Malformed lines return an error rather than panicking,
because a live recording can be truncated mid-write and one bad line must not cost the whole read.

`encode_line` is canonicalizing, not byte-exact: encoding a decoded event and re-parsing it
reproduces the same typed value (`parse_line(seq, &encode_line(&event)?)? == event`), but it is not
a general-purpose codec and does not promise to reproduce an originally-captured line's exact
bytes — the type model already discarded some of what the wire format carries (unit names, most
unit-state sub-fields, the original UTC offset) before encoding ever runs. `EventBody::Unknown` and
`::Invalid` are unrepresentable and return an error rather than emitting text.

## Scope

Decoding and encoding only. Aggregation, validation, anonymization, encounter segmentation, and
resolving the game's numeric ids against a catalog all live in the consumer — each is a policy
decision an application makes, and folding them in here would narrow what the crate is good for.

## Licence

MIT.

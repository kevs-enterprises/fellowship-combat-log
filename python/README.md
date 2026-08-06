# fellowship-combat-log (Python)

PyO3 bindings for [`fellowship-combat-log`](../rust): decodes
[Fellowship](https://coffeestain.com/game/fellowship/)'s Advanced Combat Log into typed events.
Log text in, dicts out — nothing else. The decoding logic lives once, in Rust; this package is a
thin native-extension binding over it, not a reimplementation.

## Install

```bash
pip install fellowship-combat-log
```

Prebuilt wheels aside, this package builds from source with [maturin](https://www.maturin.rs/) and
needs a Rust toolchain to do so.

## Usage

```python
import fellowship_combat_log as fcl

for i, line in enumerate(open("session.log"), start=1):
    line = line.rstrip("\n")
    if not line:
        continue
    try:
        event = fcl.parse_line(i, line)
    except ValueError:
        continue  # a truncated or unrecognized line, never an exception you can't catch

    if event["body"]["type"] == "DamageHeal":
        body = event["body"]
        print(event["instant"]["utc_ms"], body["applied"], body["result"])

combatants = fcl.list_combatants(open("session.log").read())
for c in combatants:
    print(c["name"], c["info"]["hero_id"], c["info"]["item_level"])

print(fcl.version())
```

## API

- `parse_line(seq: int, line: str) -> Event` — decode one log line. `seq` is the file line number
  (the ordering tiebreaker for sub-millisecond ties). Raises `ValueError` with a clear message on
  a malformed line; never returns a result-union.
- `list_combatants(log: str) -> list[Combatant]` — every combatant a log mentions, in first-seen
  order, each with its latest equipped-gear snapshot. Never raises: a malformed or unparseable
  line is skipped rather than aborting the scan.
- `version() -> str` — the decoder's version, for a consumer that records which decoder produced a
  result and wants to detect staleness after an upgrade.

Every returned value is a plain `dict`/`list`/`str`/`int`/`float`/`bool` — no custom classes.
`Event`, `Combatant`, `Guid`, and every other shape are `TypedDict`/`Literal` definitions in
[`python/fellowship_combat_log/__init__.pyi`](python/fellowship_combat_log/__init__.pyi) for
`mypy`/`pyright`, matching the JSON these functions actually produce field for field:

- a Rust enum with data-carrying variants (`Guid`, `CastPhase`, `EncounterPhase`, the `Event`
  body) becomes a `Union` of `TypedDict`s discriminated by a `"type"` key holding the Rust variant
  name (e.g. `{"type": "DamageHeal", "kind": "AbilityDamage", ...}`).
- a unit-only Rust enum (`ResultTier`, `School`, `Polarity`, ...) becomes a bare `Literal[...]`
  string union (e.g. `result: Literal["Hit", "CriticalStrike", ...]`).
- every field keeps the decoder's own snake_case name — nothing is renamed for Python.

## Scope

Decoding only, same as the Rust crate it binds. Aggregation, validation, anonymization, encounter
segmentation, and resolving the game's numeric ids against a catalog all live in the consumer.

## Build from source

```bash
cd public/python
python3 -m venv .venv
source .venv/bin/activate
pip install maturin pytest
maturin develop          # builds the extension and installs it into the active venv
pytest                   # decodes the fixtures under ../rust/tests/fixtures and checks the shapes
```

Formatting and lints (from the workspace root, `public/`):

```bash
cargo fmt --check -p fellowship-combat-log-py
cargo clippy -p fellowship-combat-log-py --all-targets -- -D warnings
```

## Licence

MIT.

"""Python bindings for fellowship-combat-log: decode Fellowship's Advanced Combat Log.

Log text in, typed events out — nothing else. This package binds the same dependency-free Rust
decoder every other language binding uses. Every function here hands back a plain `dict`/`list`
value; see `__init__.pyi` for the precise `TypedDict`/`Literal`-discriminated shapes those values
have.
"""

from .fellowship_combat_log import list_combatants, parse_line, version

__all__ = ["list_combatants", "parse_line", "version"]

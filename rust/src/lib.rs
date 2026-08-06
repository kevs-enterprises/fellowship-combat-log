//! Advanced Combat Log decoding and encoding: log text in, typed events out —
//! and typed events in, log text out.
//!
//! A wasm32-compatible library: no `std::fs`, no `SystemTime`, no native-only
//! crates, and no dependencies at all. All I/O belongs to the caller, which is
//! what lets the same decoder/encoder run in a browser and in a native tool.
//!
//! The five modules form a closed dependency graph, `timestamp -> event ->
//! {parse, encode} -> combatants`, with no edge leaving the set. That closure is
//! what makes the crate consumable on its own: a caller wanting "which abilities
//! fired, when, from whom" takes this and nothing else. Aggregation, validation,
//! encounter segmentation, and resolving ids against a catalog are deliberately
//! left to the consumer, because each is a policy decision an application makes.
//!
//! `parse_line` is line-oriented, so decoding a log as it is written needs no
//! different API here — only a caller that owns the file handle. `encode_line`
//! is canonicalizing, not byte-exact: encoding a decoded event and re-parsing it
//! reproduces the same typed value, but encoding is not a general-purpose,
//! format-agnostic codec and does not promise to reproduce an original line's
//! exact bytes.

pub mod combatants;
pub mod encode;
pub mod event;
pub mod parse;
pub mod timestamp;

/// This decoder's version, for a consumer that records which decoder produced a
/// result and wants to detect staleness after an upgrade.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

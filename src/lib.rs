//! Advanced Combat Log decoding: log text in, typed events out.
//!
//! A wasm32-compatible library: no `std::fs`, no `SystemTime`, no native-only
//! crates, and no dependencies at all. All I/O belongs to the caller, which is
//! what lets the same decoder run in a browser and in a native tool.
//!
//! The four modules form a closed dependency chain, `timestamp -> event -> parse
//! -> combatants`, with no edge leaving the set. That closure is what makes the
//! crate consumable on its own: a caller wanting "which abilities fired, when,
//! from whom" takes this and nothing else. Aggregation, validation, encounter
//! segmentation, and resolving ids against a catalog are deliberately left to the
//! consumer, because each is a policy decision an application makes.
//!
//! `parse_line` is line-oriented, so decoding a log as it is written needs no
//! different API here — only a caller that owns the file handle.

pub mod combatants;
pub mod event;
pub mod parse;
pub mod timestamp;

/// This decoder's version, for a consumer that records which decoder produced a
/// result and wants to detect staleness after an upgrade.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

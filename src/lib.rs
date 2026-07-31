//! Advanced Combat Log decoding: log text in, typed events out.
//!
//! A wasm32-compatible library: no `std::fs`, no `SystemTime`, no
//! native-only crates, and no dependencies at all. All I/O belongs to a caller —
//! `the native CLI` owns it in this repository.
//!
//! The four modules form a closed dependency chain, `timestamp -> event -> parse
//! -> combatants`, with no edge leaving the set. That closure is what makes this
//! crate consumable on its own: a caller wanting "which abilities fired, when,
//! from whom" takes this and nothing else. What deliberately stays behind in
//! `the pipeline crate` is the the design notes Log-extract pipeline (`extract`, `validation`,
//! `provenance`, `emit`, `anonymize`, `corpus`), the `name_join` catalog bridge,
//! and `segment` — segmentation reads `Extract` and `ExtractGrade`, so it sits on
//! top of that pipeline rather than under it.
//!
//! `parse_line` is line-oriented, so incremental live tailing needs no refactor
//! here — only a caller that owns the file handle.

pub mod combatants;
pub mod event;
pub mod parse;
pub mod timestamp;

/// This decoder's version, stamped into a Log extract's provenance header so
/// staleness against a parser change is detectable (mirrors `a version stamp elsewhere`).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

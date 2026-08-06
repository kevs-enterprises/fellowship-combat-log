//! `wasm-bindgen` bindings for `fellowship-combat-log`: three exports —
//! `parseLine`, `listCombatants`, `version` — that hand a JS/TS caller the
//! decoded event model as plain JSON-shaped objects (see `bridge` for the
//! mirror types and the JSON shape they produce).
//!
//! The `.d.ts` `wasm-bindgen` emits for these can only describe a
//! `JsValue`-returning export as `any`; the precise discriminated-union types a
//! consumer actually sees come from the hand-written declarations the
//! TypeScript entry point wraps this raw module with.

mod bridge;

use bridge::{CombatantJson, EventJson, describe_parse_error, to_js};
use fellowship_combat_log::combatants::list_combatants;
use fellowship_combat_log::parse::parse_line;
use wasm_bindgen::prelude::*;

/// Decode one v8 combat-log line. `seq` is the file line number (1-based, the
/// same tiebreaker `fellowship_combat_log` uses internally) — the caller
/// supplies it because a single line carries no notion of its own position in
/// the file.
///
/// Throws a JS `Error` with a human-readable message on a malformed line,
/// rather than returning some ok/error union, so a caller can `try`/`catch`
/// (or let it propagate) the same way any other JS decoding failure works.
#[wasm_bindgen(js_name = parseLine)]
pub fn parse_line_js(seq: u32, line: &str) -> Result<JsValue, JsValue> {
    match parse_line(seq, line) {
        Ok(event) => to_js(&EventJson::from(&event)),
        Err(error) => Err(JsValue::from(js_sys::Error::new(&describe_parse_error(
            seq, &error,
        )))),
    }
}

/// List every combatant a log mentions, in first-seen order, each carrying its
/// latest `COMBATANT_INFO` gear snapshot.
///
/// Malformed or unparseable lines are skipped rather than failing the scan
/// (same as the underlying `fellowship_combat_log::combatants::list_combatants`),
/// so this never throws.
#[wasm_bindgen(js_name = listCombatants)]
pub fn list_combatants_js(log: &str) -> Result<JsValue, JsValue> {
    let combatants: Vec<CombatantJson> = list_combatants(log)
        .iter()
        .map(CombatantJson::from)
        .collect();
    to_js(&combatants)
}

/// This decoder's version, for a consumer that records which decoder produced
/// a result and wants to detect staleness after an upgrade.
#[wasm_bindgen]
pub fn version() -> String {
    fellowship_combat_log::version().to_string()
}
